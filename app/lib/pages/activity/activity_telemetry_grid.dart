import 'package:flutter/material.dart';

import '../../models/activity_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class ActivityTelemetryGrid extends StatelessWidget {
  final StateProvider state;

  const ActivityTelemetryGrid({super.key, required this.state});

  @override
  Widget build(BuildContext context) {
    final totalEvents = state.events.length;
    final recentMinutes = state.events.isNotEmpty
        ? DateTime.now()
              .difference(state.events.last.timestamp)
              .inMinutes
              .clamp(1, 60)
        : 1;
    final rate = totalEvents > 0
        ? (totalEvents / recentMinutes).toStringAsFixed(1)
        : '0.0';
    final rateProgress = (totalEvents / 30.0).clamp(0.0, 1.0);

    final autoCommits = state.events
        .where(
          (e) =>
              e.message.toLowerCase().contains('grab') ||
              e.message.toLowerCase().contains('added') ||
              e.message.toLowerCase().contains('download') ||
              e.message.toLowerCase().contains('commit'),
        )
        .length;
    final ratio = totalEvents > 0 ? (autoCommits / totalEvents) : 1.0;
    final acceptancePercent = '${(ratio * 100).toStringAsFixed(1)}%';

    final filtered = state.events
        .where(
          (e) =>
              e.message.toLowerCase().contains('reject') ||
              e.message.toLowerCase().contains('discard') ||
              e.message.toLowerCase().contains('ignore') ||
              e.message.toLowerCase().contains('filtered'),
        )
        .length;
    final filteredProgress = (filtered / 20.0).clamp(0.0, 1.0);

    final errors = state.events
        .where((e) => e.level == ActivityLevel.error)
        .length;
    final errorProgress = (errors / 10.0).clamp(0.0, 1.0);

    return LayoutBuilder(
      builder: (context, constraints) {
        final cardWidth = (constraints.maxWidth - 36) / 4;
        return Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            _buildTelemetryCard(
              title: 'INGESTION RATE',
              icon: Icons.bolt_rounded,
              iconColor: ObsidianColors.tertiary,
              value: rate,
              unit: 'events/min',
              progress: rateProgress,
              progressColor: ObsidianColors.tertiary,
              width: cardWidth,
            ),
            _buildTelemetryCard(
              title: 'DECISION ACCEPTANCE',
              icon: Icons.rule_folder_rounded,
              iconColor: ObsidianColors.primary,
              value: acceptancePercent,
              unit: 'auto-commit',
              progress: ratio.clamp(0.0, 1.0),
              progressColor: ObsidianColors.primary,
              width: cardWidth,
            ),
            _buildTelemetryCard(
              title: 'DISCARDED RELEASES',
              icon: Icons.filter_list_off_rounded,
              iconColor: ObsidianColors.secondary,
              value: '$filtered',
              unit: 'filtered',
              progress: filteredProgress,
              progressColor: ObsidianColors.secondary,
              width: cardWidth,
            ),
            _buildTelemetryCard(
              title: 'ERROR EXCLUSIONS',
              icon: Icons.error_outline_rounded,
              iconColor: ObsidianColors.error,
              value: '$errors',
              unit: 'unresolved',
              progress: errorProgress,
              progressColor: ObsidianColors.error,
              width: cardWidth,
            ),
          ],
        );
      },
    );
  }

  Widget _buildTelemetryCard({
    required String title,
    required IconData icon,
    required Color iconColor,
    required String value,
    required String unit,
    required double progress,
    required Color progressColor,
    required double width,
  }) {
    return Container(
      width: width,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                title,
                style: ObsidianTypography.labelBadge(
                  color: ObsidianColors.outline,
                ),
              ),
              Icon(icon, size: 16, color: iconColor),
            ],
          ),
          const SizedBox(height: 6),
          Row(
            crossAxisAlignment: CrossAxisAlignment.baseline,
            textBaseline: TextBaseline.alphabetic,
            children: [
              Text(value, style: ObsidianTypography.headlineMd()),
              const SizedBox(width: 6),
              Text(
                unit,
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.outline,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          ClipRRect(
            borderRadius: BorderRadius.circular(4),
            child: LinearProgressIndicator(
              value: progress.clamp(0.0, 1.0),
              backgroundColor: ObsidianColors.surfaceContainer,
              valueColor: AlwaysStoppedAnimation<Color>(progressColor),
              minHeight: 4,
            ),
          ),
        ],
      ),
    );
  }
}
