import 'dart:io';

import 'package:flutter/material.dart';

import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'ingestion_progress.dart';

/// Structured error description for ingestion stage failures.
class IngestionErrorInfo {
  final String stageName;
  final String failureSummary;
  final String hostPort;
  final String remediation;
  final String rawDetails;
  final bool isLocation;

  const IngestionErrorInfo({
    required this.stageName,
    required this.failureSummary,
    required this.hostPort,
    required this.remediation,
    required this.rawDetails,
    this.isLocation = false,
  });

  factory IngestionErrorInfo.from({
    IngestionStage? stage,
    String? rawError,
    String? dbPath,
    String? jikanBaseUrl,
  }) {
    final raw = rawError ?? 'Unknown error';
    final lower = raw.toLowerCase();

    String extractHost(String defaultHost) {
      final rawHost = RegExp(r'https?://[a-zA-Z0-9\.\:]+')
          .firstMatch(raw)
          ?.group(0);
      final candidate = rawHost ?? defaultHost;
      final uri = Uri.tryParse(candidate);
      if (uri != null && uri.host.isNotEmpty) {
        final port = uri.hasPort
            ? uri.port
            : (uri.scheme == 'https' ? 443 : 80);
        return '${uri.host}:$port';
      }
      return defaultHost;
    }

    String? pathFromRaw;
    final dbMatch = RegExp(r'(/[\w\.\-]+)+/genrl\.db').firstMatch(raw);
    if (dbMatch != null) {
      pathFromRaw = dbMatch.group(0);
    }

    final resolvedDbPath = (dbPath != null && dbPath.isNotEmpty)
        ? (dbPath.endsWith('.db') ? dbPath : '$dbPath/genrl.db')
        : (pathFromRaw ??
              (Platform.environment['GENRL_REQUIRE_SCRATCH'] != null
                  ? '/tmp/genrl-scratch/genrl.db'
                  : (Platform.isLinux
                        ? '${Platform.environment['HOME']}/.local/share/genrl-downloader/genrl.db'
                        : 'genrl.db')));

    // 1. Stage 1: MAL unreachable
    if (stage == IngestionStage.malSync ||
        (stage == null &&
            (lower.contains('myanimelist') ||
                lower.contains('mal sync') ||
                lower.contains('oauth2/token')))) {
      String malFallback = 'https://api.myanimelist.net:443';
      final match = RegExp(r'https?://[a-zA-Z0-9\.\:]+').firstMatch(raw);
      if (match != null) {
        malFallback = match.group(0)!;
      }
      return IngestionErrorInfo(
        stageName: 'Stage 1: MAL Library Sync',
        failureSummary: 'MyAnimeList API connection failed',
        hostPort: extractHost(malFallback),
        remediation: 'check network connection or MyAnimeList API status',
        rawDetails: raw,
      );
    }

    // 2. Stage 2: Jikan failure
    if (stage == IngestionStage.metadata ||
        (stage == null && (lower.contains('429') || lower.contains('jikan')))) {
      final isTimeout =
          lower.contains('timed out') || lower.contains('timeout');
      final isRateLimit =
          RegExp(r'\b(?:http|status)\s*429\b|\(429\)').hasMatch(lower) ||
          lower.contains('rate limit');
      final isBreaker =
          lower.contains('circuit breaker') || lower.contains('breaker open');
      final serverError = RegExp(
        r'\b(?:http|status)\s*(5\d{2})\b',
        caseSensitive: false,
      ).firstMatch(raw)?.group(1);
      final summary = isTimeout
          ? 'Jikan request timed out'
          : isRateLimit
          ? 'Jikan API rate limit (HTTP 429 Too Many Requests)'
          : isBreaker
          ? 'Jikan requests paused while service recovers'
          : serverError != null
          ? 'Jikan API server error (HTTP $serverError)'
          : 'Jikan metadata request failed';
      String jikanFallback = 'https://api.jikan.moe:443';
      if (jikanBaseUrl != null && jikanBaseUrl.isNotEmpty) {
        jikanFallback = jikanBaseUrl;
      }
      return IngestionErrorInfo(
        stageName: 'Stage 2: Show Metadata Resolution',
        failureSummary: summary,
        hostPort: extractHost(jikanFallback),
        remediation: isRateLimit
            ? 'wait for the server retry time before retrying'
            : 'check Jikan connectivity, or connect MAL for metadata fallback',
        rawDetails: raw,
      );
    }

    // 3. Stage 4: Create Jobs failure
    if (stage == IngestionStage.createJobs) {
      final isDbError =
          lower.contains('sqlite') ||
          lower.contains('readonly') ||
          lower.contains('database') ||
          lower.contains('genrl.db');
      return IngestionErrorInfo(
        stageName: 'Stage 4: Create Torrent Jobs',
        failureSummary: isDbError
            ? 'SQLite database access error'
            : 'Job creation and plan confirmation failed',
        hostPort: resolvedDbPath,
        remediation: isDbError
            ? 'check SQLite database permissions or unlock genrl.db'
            : 'verify queue state and database permissions',
        rawDetails: raw,
        isLocation: true,
      );
    }

    // 4. Stage 3: Queue Planning failure
    return IngestionErrorInfo(
      stageName: 'Stage 3: Queue Planning & Rules',
      failureSummary: 'Queue planning evaluation error',
      hostPort: resolvedDbPath,
      remediation: 'verify queue state and show rules',
      rawDetails: raw,
      isLocation: true,
    );
  }
}

/// Card presenting stage, failure summary, target host/port, remediation fix, and raw details.
class IngestionErrorCard extends StatelessWidget {
  final IngestionErrorInfo info;

  const IngestionErrorCard({super.key, required this.info});

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.symmetric(vertical: 12),
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: ObsidianColors.error.withOpacity(0.08),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: ObsidianColors.error.withOpacity(0.4)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(
                Icons.error_outline_rounded,
                color: ObsidianColors.error,
                size: 20,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  info.stageName,
                  style: ObsidianTypography.titleSm().copyWith(
                    color: ObsidianColors.error,
                    fontWeight: FontWeight.bold,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          _buildRow('What failed', info.failureSummary),
          _buildRow(
            info.isLocation ? 'Location' : 'Host / Port',
            info.hostPort,
          ),
          _buildRow('Fix', info.remediation),
          const SizedBox(height: 8),
          Text(
            'Details:',
            style: ObsidianTypography.labelBadge(
              color: ObsidianColors.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 4),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(6),
              border: Border.all(color: ObsidianColors.border),
            ),
            child: SelectableText(
              info.rawDetails,
              style: ObsidianTypography.monoCode(
                color: ObsidianColors.onSurface,
              ).copyWith(fontSize: 11),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildRow(String label, String value) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 90,
            child: Text(
              '$label:',
              style: ObsidianTypography.labelBadge(
                color: ObsidianColors.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(
            child: Text(
              value,
              style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface)
                  .copyWith(fontWeight: FontWeight.w500),
            ),
          ),
        ],
      ),
    );
  }
}
