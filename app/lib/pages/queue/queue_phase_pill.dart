import 'package:flutter/material.dart';

import '../../models/queue_item.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../widgets/loading/inline_spinner.dart';

class QueuePhasePill extends StatelessWidget {
  final QueueItem item;

  const QueuePhasePill({super.key, required this.item});

  @override
  Widget build(BuildContext context) {
    Color bg = ObsidianColors.primaryContainer.withOpacity(0.2);
    Color fg = ObsidianColors.primary;
    String label = 'Downloading ${(item.progress * 100).toInt()}%';

    if (item.state == QueueState.finalizing) {
      bg = ObsidianColors.tertiaryContainer.withOpacity(0.2);
      fg = ObsidianColors.tertiary;
      label = 'Hardlinking Media';
    } else if (item.state == QueueState.queued) {
      bg = ObsidianColors.surfaceContainerHighest;
      fg = ObsidianColors.outline;
      label = 'Searching Indexers';
    } else if (item.state == QueueState.blocked) {
      bg = ObsidianColors.error.withOpacity(0.2);
      fg = ObsidianColors.error;
      label = item.error?.isNotEmpty == true ? item.error! : 'Blocked';
    } else if (item.state == QueueState.stalled) {
      bg = ObsidianColors.error.withOpacity(0.3);
      fg = ObsidianColors.error;
      label = item.error?.isNotEmpty == true
          ? item.error!
          : 'Swarm Stalled (0 seeds)';
    } else if (item.state == QueueState.failed) {
      bg = ObsidianColors.error.withOpacity(0.3);
      fg = ObsidianColors.error;
      label = item.error?.isNotEmpty == true ? item.error! : 'Failed';
    } else if (item.seeders <= 1) {
      bg = ObsidianColors.secondaryContainer.withOpacity(0.4);
      fg = ObsidianColors.secondary;
      label = 'Pick Alternative (1 seed)';
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (item.state == QueueState.queued) ...[
            const InlineSpinner(size: 9, color: ObsidianColors.outline),
            const SizedBox(width: 4),
          ],
          Flexible(
            child: Text(
              label,
              style: ObsidianTypography.monoCode(
                color: fg,
                fontSize: 10,
                fontWeight: FontWeight.w500,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}
