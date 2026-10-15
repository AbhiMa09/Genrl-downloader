import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'cyber_loader.dart';
import 'inline_spinner.dart';

enum IngestionStage {
  malSync,
  metadata,
  planQueue,
  createJobs,
  completed,
  failed,
}

/// Shared IngestionProgress widget adhering to the Stitch design specifications.
/// Features a headline, concentric loader, dual-segmented progress bar,
/// 4 real core stage tiles, telemetry counts/ETA, and action controls.
class IngestionProgress extends StatelessWidget {
  final IngestionStage stage;
  final int processedCount;
  final int totalCount;
  final int? etaSeconds;
  final bool isPaused;
  final String? errorMessage;
  final VoidCallback? onPauseToggle;
  final VoidCallback? onBackground;
  final VoidCallback? onViewTelemetry;
  final VoidCallback? onRetry;

  const IngestionProgress({
    super.key,
    required this.stage,
    this.processedCount = 0,
    this.totalCount = 0,
    this.etaSeconds,
    this.isPaused = false,
    this.errorMessage,
    this.onPauseToggle,
    this.onBackground,
    this.onViewTelemetry,
    this.onRetry,
  });

  String get _headline {
    switch (stage) {
      case IngestionStage.malSync:
        return 'Synchronizing MyAnimeList Library...';
      case IngestionStage.metadata:
        return 'Resolving Show Metadata & Posters...';
      case IngestionStage.planQueue:
        return 'Analyzing Release Schedule & Planning Queue...';
      case IngestionStage.createJobs:
        return 'Committing Initial Queue Jobs...';
      case IngestionStage.completed:
        return 'Ingestion Complete — Ready to Launch!';
      case IngestionStage.failed:
        return 'Ingestion Stage Interrupted';
    }
  }

  String get _subhead {
    switch (stage) {
      case IngestionStage.malSync:
        return 'Fetching watching and plan-to-watch series from MAL API.';
      case IngestionStage.metadata:
        return 'Downloading synopses and high-res seasonal posters.';
      case IngestionStage.planQueue:
        return 'Evaluating episode missing states against active rules.';
      case IngestionStage.createJobs:
        return 'Enqueuing initial downloads into local pipeline scheduler.';
      case IngestionStage.completed:
        return 'All initial manifests verified. Background workers active.';
      case IngestionStage.failed:
        return errorMessage ?? 'An error occurred during stage execution.';
    }
  }

  @override
  Widget build(BuildContext context) {
    final progress = totalCount > 0 ? (processedCount / totalCount).clamp(0.0, 1.0) : 0.0;
    final percent = (progress * 100).toStringAsFixed(1);

    return Focus(
      autofocus: true,
      onKeyEvent: (node, event) {
        if (event is KeyDownEvent &&
            event.logicalKey == LogicalKeyboardKey.keyB &&
            HardwareKeyboard.instance.isControlPressed) {
          onBackground?.call();
          return KeyEventResult.handled;
        }
        return KeyEventResult.ignored;
      },
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 680),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              // Concentric Radar Rings
              CyberLoader(
                size: 160,
                isAnimating: !isPaused && stage != IngestionStage.completed && stage != IngestionStage.failed,
              ),
              const SizedBox(height: 20),

              // Headline & Description
              Text(
                _headline,
                style: ObsidianTypography.headlineMd(),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 6),
              Text(
                _subhead,
                style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 20),

              // Dual-Segmented Progress Metrics
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(
                    '$processedCount / $totalCount episodes verified',
                    style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
                  ),
                  Text(
                    '$percent%',
                    style: ObsidianTypography.monoCode(color: ObsidianColors.primary).copyWith(fontWeight: FontWeight.bold),
                  ),
                ],
              ),
              const SizedBox(height: 8),

              // Progress Bar Track
              Container(
                height: 10,
                width: double.infinity,
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(5),
                ),
                child: Stack(
                  children: [
                    FractionallySizedBox(
                      widthFactor: progress,
                      child: Container(
                        decoration: BoxDecoration(
                          color: ObsidianColors.primary,
                          borderRadius: BorderRadius.circular(5),
                          boxShadow: [
                            BoxShadow(
                              color: ObsidianColors.primary.withOpacity(0.5),
                              blurRadius: 8,
                            ),
                          ],
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 8),

              // Time remaining & Sub-counts
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Row(
                    children: [
                      Container(width: 8, height: 8, decoration: const BoxDecoration(color: ObsidianColors.primary, shape: BoxShape.circle)),
                      const SizedBox(width: 6),
                      Text('Committed: $processedCount', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                    ],
                  ),
                  Text(
                    etaSeconds != null ? 'Time remaining: ~${etaSeconds}s' : 'Estimating...',
                    style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
                  ),
                ],
              ),
              const SizedBox(height: 24),

              // 4 Stage Tiles
              Row(
                children: [
                  Expanded(child: _buildStageTile(1, 'MAL Sync', IngestionStage.malSync)),
                  const SizedBox(width: 8),
                  Expanded(child: _buildStageTile(2, 'Metadata', IngestionStage.metadata)),
                  const SizedBox(width: 8),
                  Expanded(child: _buildStageTile(3, 'Plan Queue', IngestionStage.planQueue)),
                  const SizedBox(width: 8),
                  Expanded(child: _buildStageTile(4, 'Create Jobs', IngestionStage.createJobs)),
                ],
              ),
              const SizedBox(height: 24),

              // Action Buttons
              _buildActionRow(),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildStageTile(int stepNumber, String title, IngestionStage tileStage) {
    final isDone = stage.index > tileStage.index || stage == IngestionStage.completed;
    final isActive = stage == tileStage;
    final isFailed = isActive && stage == IngestionStage.failed;

    Color bgColor = ObsidianColors.surfaceContainerHigh;
    Color iconColor = ObsidianColors.outline;
    Widget icon = Icon(Icons.circle_outlined, size: 14, color: iconColor);

    if (isDone) {
      bgColor = ObsidianColors.surfaceContainerHigh;
      iconColor = ObsidianColors.tertiary;
      icon = Icon(Icons.check_circle_rounded, size: 14, color: iconColor);
    } else if (isFailed) {
      bgColor = ObsidianColors.errorContainer.withOpacity(0.4);
      iconColor = ObsidianColors.error;
      icon = Icon(Icons.error_outline_rounded, size: 14, color: iconColor);
    } else if (isActive) {
      bgColor = ObsidianColors.secondaryContainer.withOpacity(0.3);
      iconColor = ObsidianColors.secondary;
      icon = const InlineSpinner(size: 12, color: ObsidianColors.secondary);
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      decoration: BoxDecoration(
        color: bgColor,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: isActive ? ObsidianColors.secondary.withOpacity(0.4) : Colors.transparent,
          width: 0.5,
        ),
      ),
      child: Row(
        children: [
          icon,
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  '$stepNumber. $title',
                  style: ObsidianTypography.labelBadge(
                    color: isActive ? ObsidianColors.secondary : (isDone ? ObsidianColors.onSurface : ObsidianColors.outline),
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                Text(
                  isDone ? 'Completed' : (isActive ? 'Running' : 'Queued'),
                  style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 10),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildActionRow() {
    if (stage == IngestionStage.failed) {
      return Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          ElevatedButton.icon(
            onPressed: onRetry,
            icon: const Icon(Icons.refresh_rounded, size: 16),
            label: const Text('Retry Stage'),
            style: ElevatedButton.styleFrom(
              backgroundColor: ObsidianColors.error,
              foregroundColor: ObsidianColors.onError,
            ),
          ),
        ],
      );
    }

    return Wrap(
      spacing: 12,
      runSpacing: 8,
      alignment: WrapAlignment.center,
      children: [
        OutlinedButton.icon(
          onPressed: onPauseToggle,
          icon: Icon(isPaused ? Icons.play_arrow_rounded : Icons.pause_rounded, size: 16),
          label: Text(isPaused ? 'Resume' : 'Pause'),
          style: OutlinedButton.styleFrom(
            side: const BorderSide(color: ObsidianColors.border),
            foregroundColor: ObsidianColors.onSurfaceVariant,
          ),
        ),
        OutlinedButton.icon(
          onPressed: onBackground,
          icon: const Icon(Icons.splitscreen_rounded, size: 16),
          label: const Text('Run in background (Ctrl+B)'),
          style: OutlinedButton.styleFrom(
            side: const BorderSide(color: ObsidianColors.border),
            foregroundColor: ObsidianColors.onSurfaceVariant,
          ),
        ),
        TextButton.icon(
          onPressed: onViewTelemetry,
          icon: const Icon(Icons.monitor_heart_outlined, size: 16),
          label: const Text('View telemetry'),
          style: TextButton.styleFrom(
            foregroundColor: ObsidianColors.tertiary,
          ),
        ),
      ],
    );
  }
}
