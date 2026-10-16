import 'package:flutter/material.dart';

import '../../theme/colors.dart';
import '../../theme/typography.dart';

class InboxHeader extends StatelessWidget {
  final int pendingCount;
  final TextEditingController searchController;
  final VoidCallback onSearchChanged;
  final VoidCallback onBatchResolve;

  const InboxHeader({
    super.key,
    required this.pendingCount,
    required this.searchController,
    required this.onSearchChanged,
    required this.onBatchResolve,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Wrap(
          alignment: WrapAlignment.spaceBetween,
          crossAxisAlignment: WrapCrossAlignment.center,
          spacing: 16,
          runSpacing: 10,
          children: [
            Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text('Inbox', style: ObsidianTypography.headlineLg()),
                    const SizedBox(width: 10),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 2,
                      ),
                      decoration: BoxDecoration(
                        color: ObsidianColors.primaryContainer.withOpacity(0.2),
                        borderRadius: BorderRadius.circular(12),
                      ),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Container(
                            width: 6,
                            height: 6,
                            decoration: const BoxDecoration(
                              color: ObsidianColors.primary,
                              shape: BoxShape.circle,
                            ),
                          ),
                          const SizedBox(width: 6),
                          Text(
                            '$pendingCount Decisions Pending',
                            style: ObsidianTypography.monoMetric(
                              color: ObsidianColors.primary,
                              fontWeight: FontWeight.bold,
                              fontSize: 11,
                            ),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 6,
                        vertical: 2,
                      ),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHigh,
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        'parser-queue: halted',
                        style: ObsidianTypography.monoCode(
                          color: ObsidianColors.outline,
                          fontSize: 10,
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  'Automated ingestion pipeline pauses requiring user intervention or rule clarification before commit.',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.outline,
                  ),
                ),
              ],
            ),

            // Search & Batch Actions
            Wrap(
              spacing: 8,
              runSpacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                Container(
                  width: 220,
                  height: 36,
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLowest,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: ObsidianColors.border),
                  ),
                  child: Row(
                    children: [
                      const Icon(
                        Icons.filter_list_rounded,
                        size: 16,
                        color: ObsidianColors.outline,
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: TextField(
                          controller: searchController,
                          style: ObsidianTypography.bodySm(
                            color: ObsidianColors.onSurface,
                          ),
                          decoration: InputDecoration(
                            hintText: 'Filter decisions...',
                            hintStyle: ObsidianTypography.bodySm(
                              color: ObsidianColors.outline,
                            ),
                            border: InputBorder.none,
                            isDense: true,
                          ),
                          onChanged: (_) => onSearchChanged(),
                        ),
                      ),
                    ],
                  ),
                ),
                OutlinedButton.icon(
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(
                        content: Text(
                          'Notifications suppressed for 60 minutes',
                        ),
                      ),
                    );
                  },
                  icon: const Icon(
                    Icons.snooze_rounded,
                    size: 14,
                    color: ObsidianColors.outline,
                  ),
                  label: Text(
                    'Ignore 1h',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.onSurfaceVariant,
                      fontSize: 11,
                    ),
                  ),
                  style: OutlinedButton.styleFrom(
                    backgroundColor: ObsidianColors.surfaceContainerHigh,
                    side: const BorderSide(color: ObsidianColors.border),
                    padding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 8,
                    ),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                ),
                ElevatedButton.icon(
                  onPressed: onBatchResolve,
                  icon: const Icon(Icons.done_all_rounded, size: 16),
                  label: Text(
                    'Batch Resolve All',
                    style: ObsidianTypography.titleSm(
                      color: ObsidianColors.onPrimary,
                      fontSize: 12,
                    ),
                  ),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: ObsidianColors.primary,
                    foregroundColor: ObsidianColors.onPrimary,
                    elevation: 0,
                    padding: const EdgeInsets.symmetric(
                      horizontal: 14,
                      vertical: 8,
                    ),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                ),
              ],
            ),
          ],
        ),
      ],
    );
  }
}
