import 'package:flutter/material.dart';

import '../theme/colors.dart';
import '../theme/typography.dart';

class MetricCard extends StatelessWidget {
  final String title;
  final IconData icon;
  final String primaryValue;
  final String subtitle;
  final String detailLine;
  final double progress; // 0.0 to 1.0
  final Color accentColor;

  const MetricCard({
    super.key,
    required this.title,
    required this.icon,
    required this.primaryValue,
    required this.subtitle,
    required this.detailLine,
    required this.progress,
    this.accentColor = ObsidianColors.primary,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border, width: 1),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header Row
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text(
                  title.toUpperCase(),
                  style: ObsidianTypography.monoCode(
                    color: ObsidianColors.onSurfaceVariant,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 6),
              Icon(icon, size: 18, color: accentColor),
            ],
          ),
          const SizedBox(height: 12),

          // Primary Value + Subtitle
          Row(
            crossAxisAlignment: CrossAxisAlignment.baseline,
            textBaseline: TextBaseline.alphabetic,
            children: [
              Flexible(
                child: FittedBox(
                  fit: BoxFit.scaleDown,
                  alignment: Alignment.centerLeft,
                  child: Text(
                    primaryValue,
                    style: ObsidianTypography.headlineXl(),
                  ),
                ),
              ),
              const SizedBox(width: 8),
              Text(
                subtitle,
                style: ObsidianTypography.titleSm(color: accentColor),
              ),
            ],
          ),
          const SizedBox(height: 6),

          // Detail line
          Text(
            detailLine,
            style: ObsidianTypography.monoCode(
              color: ObsidianColors.onSurfaceVariant,
            ),
            overflow: TextOverflow.ellipsis,
          ),
          const SizedBox(height: 12),

          // Progress bar
          ClipRRect(
            borderRadius: BorderRadius.circular(4),
            child: LinearProgressIndicator(
              value: progress.clamp(0.0, 1.0),
              backgroundColor: ObsidianColors.surfaceContainerHighest,
              valueColor: AlwaysStoppedAnimation<Color>(accentColor),
              minHeight: 4,
            ),
          ),
        ],
      ),
    );
  }
}
