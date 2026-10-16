import 'package:flutter/material.dart';

import '../../models/queue_item.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class QueueFilterBar extends StatelessWidget {
  final List<QueueItem> items;
  final String selectedFilter;
  final ValueChanged<String> onFilterChanged;
  final TextEditingController searchController;
  final VoidCallback onSearchChanged;

  const QueueFilterBar({
    super.key,
    required this.items,
    required this.selectedFilter,
    required this.onFilterChanged,
    required this.searchController,
    required this.onSearchChanged,
  });

  @override
  Widget build(BuildContext context) {
    final dlCount = items
        .where((i) => i.state == QueueState.downloading)
        .length;
    final finalCount = items
        .where(
          (i) =>
              i.state == QueueState.finalizing ||
              i.state == QueueState.completed,
        )
        .length;
    final blockedCount = items
        .where((i) => i.state == QueueState.blocked)
        .length;
    final failedCount = items
        .where((i) => i.state == QueueState.stalled)
        .length;
    final searchCount = items.where((i) => i.state == QueueState.queued).length;

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 10),
      decoration: const BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        border: Border(bottom: BorderSide(color: ObsidianColors.border)),
      ),
      child: Wrap(
        alignment: WrapAlignment.spaceBetween,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: 16,
        runSpacing: 10,
        children: [
          // Filter Tabs
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                _buildTab(
                  'all',
                  'All',
                  '${items.length}',
                  ObsidianColors.primary,
                ),
                const SizedBox(width: 6),
                _buildTab(
                  'searching',
                  'Searching',
                  '$searchCount',
                  ObsidianColors.tertiary,
                ),
                const SizedBox(width: 6),
                _buildTab(
                  'downloading',
                  'Downloading',
                  '$dlCount',
                  ObsidianColors.primary,
                ),
                const SizedBox(width: 6),
                _buildTab(
                  'finalizing',
                  'Finalizing',
                  '$finalCount',
                  ObsidianColors.onSurface,
                ),
                const SizedBox(width: 6),
                _buildTab(
                  'blocked',
                  'Blocked',
                  '$blockedCount',
                  ObsidianColors.error,
                ),
                const SizedBox(width: 6),
                _buildTab(
                  'failed',
                  'Failed',
                  '$failedCount',
                  ObsidianColors.error,
                ),
                const SizedBox(width: 6),
                _buildTab(
                  'needs_input',
                  'Needs Input',
                  '2',
                  ObsidianColors.secondary,
                ),
              ],
            ),
          ),

          // Search Input
          Container(
            width: 280,
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
                      hintText: 'Filter title, release group, hash...',
                      hintStyle: ObsidianTypography.bodySm(
                        color: ObsidianColors.outline,
                      ),
                      border: InputBorder.none,
                      isDense: true,
                    ),
                    onChanged: (_) => onSearchChanged(),
                  ),
                ),
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 4,
                    vertical: 1.5,
                  ),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHigh,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    'Ctrl+F',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.outline,
                      fontSize: 9,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildTab(String key, String label, String badge, Color badgeColor) {
    final isSelected = selectedFilter == key;
    return InkWell(
      onTap: () => onFilterChanged(key),
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: isSelected
              ? ObsidianColors.primary
              : ObsidianColors.surfaceContainer,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isSelected ? ObsidianColors.primary : ObsidianColors.border,
            width: 0.5,
          ),
        ),
        child: Text(
          '$label ($badge)',
          style: ObsidianTypography.monoCode(
            color: isSelected
                ? ObsidianColors.onPrimary
                : ObsidianColors.onSurfaceVariant,
            fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
            fontSize: 11,
          ),
        ),
      ),
    );
  }
}
