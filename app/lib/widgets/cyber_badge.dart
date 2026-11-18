import 'package:flutter/material.dart';

import '../theme/colors.dart';
import '../theme/typography.dart';

enum CyberBadgeType {
  queued,
  downloading,
  inLibrary,
  failed,
  info,
  warning,
  neutral,
}

class CyberBadge extends StatelessWidget {
  final String label;
  final CyberBadgeType? type;
  final Color? customColor;
  final IconData? icon;

  const CyberBadge({
    super.key,
    required this.label,
    this.type,
    this.customColor,
    this.icon,
  });

  factory CyberBadge.fromStatus(String? status) {
    switch (status?.toLowerCase()) {
      case 'in_library':
      case 'completed':
      case 'done':
        return const CyberBadge(
          label: 'In Library',
          type: CyberBadgeType.inLibrary,
        );
      case 'downloading':
      case 'active':
        return const CyberBadge(
          label: 'Downloading',
          type: CyberBadgeType.downloading,
        );
      case 'failed':
      case 'error':
        return const CyberBadge(label: 'Failed', type: CyberBadgeType.failed);
      case 'queued':
      default:
        return const CyberBadge(label: 'Queued', type: CyberBadgeType.queued);
    }
  }

  Color _resolveColor() {
    if (customColor != null) return customColor!;
    switch (type) {
      case CyberBadgeType.inLibrary:
        return ObsidianColors.tertiary;
      case CyberBadgeType.downloading:
        return ObsidianColors.primary;
      case CyberBadgeType.failed:
        return ObsidianColors.error;
      case CyberBadgeType.warning:
        return ObsidianColors.errorContainer;
      case CyberBadgeType.info:
        return ObsidianColors.secondary;
      case CyberBadgeType.neutral:
        return ObsidianColors.onSurfaceVariant;
      case CyberBadgeType.queued:
      default:
        return ObsidianColors.outline;
    }
  }

  @override
  Widget build(BuildContext context) {
    final color = _resolveColor();

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: color.withOpacity(0.12),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color.withOpacity(0.4), width: 0.5),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          if (icon != null) ...[
            Icon(icon, size: 10, color: color),
            const SizedBox(width: 4),
          ],
          Text(
            label.toUpperCase(),
            style: ObsidianTypography.labelBadge(color: color),
          ),
        ],
      ),
    );
  }
}
