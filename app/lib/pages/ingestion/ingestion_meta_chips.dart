import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class IngestionMetaChip extends StatelessWidget {
  final String text;

  const IngestionMetaChip({super.key, required this.text});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        text,
        style: ObsidianTypography.monoCode(
          color: ObsidianColors.onSurfaceVariant,
        ),
      ),
    );
  }
}

class IngestionPipelineStep extends StatelessWidget {
  final IconData icon;
  final Color iconColor;
  final String stepTitle;
  final String detail;
  final bool isActive;
  final bool dimmed;

  const IngestionPipelineStep({
    super.key,
    required this.icon,
    required this.iconColor,
    required this.stepTitle,
    required this.detail,
    this.isActive = false,
    this.dimmed = false,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 154,
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: isActive
            ? ObsidianColors.secondaryContainer.withOpacity(0.3)
            : ObsidianColors.surfaceContainerHigh.withOpacity(dimmed ? 0.6 : 1),
        borderRadius: BorderRadius.circular(8),
        border: isActive
            ? Border.all(
                color: ObsidianColors.secondary.withOpacity(0.5), width: 1)
            : null,
      ),
      child: Row(
        children: [
          Icon(icon, size: 16, color: iconColor),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  stepTitle,
                  style: ObsidianTypography.labelBadge(
                    color: isActive
                        ? ObsidianColors.secondary
                        : ObsidianColors.onSurface,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
                Text(
                  detail,
                  style: ObsidianTypography.monoCode(
                    color: isActive
                        ? ObsidianColors.onSurface
                        : ObsidianColors.outline,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
