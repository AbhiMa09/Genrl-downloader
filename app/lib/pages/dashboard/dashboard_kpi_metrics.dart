import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class DashboardKpiMetrics extends StatelessWidget {
  final StateProvider state;
  final DateTime? referenceTime;

  const DashboardKpiMetrics({
    super.key,
    required this.state,
    this.referenceTime,
  });

  String _currentSeasonCycle([DateTime? nowTime]) {
    final now = nowTime ?? referenceTime ?? DateTime.now();
    final season = switch (now.month) {
      1 || 2 || 3 => 'Winter',
      4 || 5 || 6 => 'Spring',
      7 || 8 || 9 => 'Summer',
      _ => 'Fall',
    };
    return '$season ${now.year}';
  }

  @override
  Widget build(BuildContext context) {
    final status = state.status;
    final totalShows = state.shows.length;
    final totalEpisodes = state.shows.fold<int>(
      0,
      (sum, show) => sum + show.totalEpisodes,
    );
    final activeQueue = state.activeQueueCount;
    final dlSpeed = status.downloadSpeedFormatted;

    final pipelineProgress = state.queue.isEmpty
        ? (activeQueue > 0 ? (status.downloadSpeedBytes > 0 ? 0.68 : 0.2) : 0.0)
        : (state.queue.fold<double>(0.0, (sum, q) => sum + q.progress) /
                  state.queue.length)
              .clamp(0.0, 1.0);
    final downloadedEpisodes = state.shows.fold<int>(
      0,
      (sum, show) => sum + show.downloadedEpisodes,
    );
    final libraryProgress = totalEpisodes > 0
        ? (downloadedEpisodes / totalEpisodes).clamp(0.0, 1.0)
        : 0.0;
    final inboxProgress = state.inboxCount > 0
        ? (1.0 / (1.0 + state.inboxCount)).clamp(0.0, 1.0)
        : 1.0;
    final now = referenceTime ?? DateTime.now();
    final seasonStartMonth = ((now.month - 1) ~/ 3) * 3 + 1;
    final seasonProgress =
        (now.difference(DateTime(now.year, seasonStartMonth, 1)).inDays / 91.0)
            .clamp(0.0, 1.0);

    return LayoutBuilder(
      builder: (context, constraints) {
        final cardWidth = constraints.maxWidth > 1150
            ? (constraints.maxWidth - 36) / 4
            : constraints.maxWidth > 700
            ? (constraints.maxWidth - 12) / 2
            : constraints.maxWidth;
        final isCompact = constraints.maxWidth <= 700;

        return Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            // Card 1: Pipeline Flow
            KpiCard(
              width: isCompact ? constraints.maxWidth : cardWidth,
              label: 'PIPELINE FLOW',
              icon: Icons.cloud_download_rounded,
              iconColor: ObsidianColors.primary,
              headline: '$activeQueue',
              subhead: 'Torrents Active',
              detail: dlSpeed.isNotEmpty && dlSpeed != '0 B/s'
                  ? '$dlSpeed • In Progress'
                  : 'Swarm Idle',
              progress: pipelineProgress,
              progressColor: ObsidianColors.primary,
            ),

            // Card 2: Library Index
            KpiCard(
              width: isCompact ? constraints.maxWidth : cardWidth,
              label: 'LIBRARY INDEX',
              icon: Icons.video_library_rounded,
              iconColor: ObsidianColors.tertiary,
              headline: '$totalShows',
              subhead: 'Series Tracked',
              detail: '$totalEpisodes Ep • ${state.status.formattedDiskSpace}',
              progress: libraryProgress,
              progressColor: ObsidianColors.tertiary,
            ),

            // Card 3: Missing Catalog
            KpiCard(
              width: isCompact ? constraints.maxWidth : cardWidth,
              label: 'ACTION INBOX',
              icon: Icons.rule_folder_rounded,
              iconColor: ObsidianColors.secondary,
              headline: '${state.inboxCount}',
              subhead: state.inboxCount > 0 ? 'Questions Pending' : 'All Clear',
              subheadColor: state.inboxCount > 0
                  ? ObsidianColors.warning
                  : ObsidianColors.success,
              detail: state.inboxCount > 0
                  ? '${state.inboxCount} action items need review'
                  : 'No questions pending in Inbox',
              progress: inboxProgress,
              progressColor: ObsidianColors.secondaryContainer,
            ),

            // Card 4: Current Cycle
            KpiCard(
              width: isCompact ? constraints.maxWidth : cardWidth,
              label: 'CURRENT CYCLE',
              icon: Icons.calendar_month_rounded,
              iconColor: ObsidianColors.primary,
              headline: _currentSeasonCycle(),
              headlineSize: 22,
              subhead:
                  '${state.shows.where((s) => s.status.toLowerCase() == "watching").length} Watching',
              detail: '${state.queue.length} in active pipeline',
              progress: seasonProgress,
              progressColor: ObsidianColors.primaryContainer,
            ),
          ],
        );
      },
    );
  }
}

class KpiCard extends StatelessWidget {
  final double width;
  final String label;
  final IconData icon;
  final Color iconColor;
  final String headline;
  final double? headlineSize;
  final String subhead;
  final Color? subheadColor;
  final String detail;
  final double progress;
  final Color progressColor;

  const KpiCard({
    super.key,
    required this.width,
    required this.label,
    required this.icon,
    required this.iconColor,
    required this.headline,
    this.headlineSize,
    required this.subhead,
    this.subheadColor,
    required this.detail,
    required this.progress,
    required this.progressColor,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      width: width,
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                label,
                style: ObsidianTypography.labelBadge(
                  color: ObsidianColors.onSurfaceVariant,
                  fontSize: 10,
                ),
              ),
              Icon(icon, size: 18, color: iconColor),
            ],
          ),
          const SizedBox(height: 8),
          Wrap(
            crossAxisAlignment: WrapCrossAlignment.end,
            spacing: 6,
            runSpacing: 2,
            children: [
              Text(
                headline,
                style: ObsidianTypography.headlineXl(
                  fontSize: headlineSize ?? 28,
                ),
              ),
              Text(
                subhead,
                style: ObsidianTypography.titleSm(
                  color: subheadColor ?? ObsidianColors.tertiary,
                  fontSize: 12,
                ),
                overflow: TextOverflow.ellipsis,
              ),
            ],
          ),
          const SizedBox(height: 4),
          Text(
            detail,
            style: ObsidianTypography.monoCode(
              color: ObsidianColors.onSurfaceVariant,
              fontSize: 10,
            ),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          const SizedBox(height: 10),
          ClipRRect(
            borderRadius: BorderRadius.circular(3),
            child: LinearProgressIndicator(
              value: progress,
              minHeight: 4,
              backgroundColor: ObsidianColors.surfaceContainerHighest,
              valueColor: AlwaysStoppedAnimation(progressColor),
            ),
          ),
        ],
      ),
    );
  }
}
