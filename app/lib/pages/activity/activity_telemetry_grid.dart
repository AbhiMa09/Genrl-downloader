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
              value: '19.4',
              unit: 'events/min',
              progress: 0.65,
              progressColor: ObsidianColors.tertiary,
              width: cardWidth,
            ),
            _buildTelemetryCard(
              title: 'DECISION ACCEPTANCE',
              icon: Icons.rule_folder_rounded,
              iconColor: ObsidianColors.primary,
              value: '94.2%',
              unit: 'auto-commit',
              progress: 0.942,
              progressColor: ObsidianColors.primary,
              width: cardWidth,
            ),
            _buildTelemetryCard(
              title: 'DISCARDED RELEASES',
              icon: Icons.filter_list_off_rounded,
              iconColor: ObsidianColors.secondary,
              value: '148',
              unit: 'filtered',
              progress: 0.38,
              progressColor: ObsidianColors.secondary,
              width: cardWidth,
            ),
            _buildTelemetryCard(
              title: 'ERROR EXCLUSIONS',
              icon: Icons.error_outline_rounded,
              iconColor: ObsidianColors.error,
              value:
                  '${state.events.where((e) => e.level == ActivityLevel.error).length}',
              unit: 'unresolved',
              progress: 0.15,
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
              Text(title,
                  style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline)),
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
              Text(unit,
                  style: ObsidianTypography.monoCode(
                      color: ObsidianColors.outline)),
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
