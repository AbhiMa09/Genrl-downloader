import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class ActivityFilterDock extends StatelessWidget {
  final String selectedLevel;
  final ValueChanged<String> onSelectLevel;
  final ValueChanged<String> onSearchChanged;
  final String selectedShow;
  final ValueChanged<String> onSelectShow;
  final String selectedTimeRange;
  final ValueChanged<String> onSelectTimeRange;
  final int totalCount;
  final int infoCount;
  final int committedCount;
  final int warnCount;
  final int errorCount;

  const ActivityFilterDock({
    super.key,
    required this.selectedLevel,
    required this.onSelectLevel,
    required this.onSearchChanged,
    required this.selectedShow,
    required this.onSelectShow,
    required this.selectedTimeRange,
    required this.onSelectTimeRange,
    required this.totalCount,
    required this.infoCount,
    required this.committedCount,
    required this.warnCount,
    required this.errorCount,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        children: [
          Row(
            children: [
              // Category chips
              _buildCategoryChip(
                'ALL',
                totalCount,
                ObsidianColors.primary,
                null,
              ),
              const SizedBox(width: 8),
              _buildCategoryChip(
                'INFO',
                infoCount,
                ObsidianColors.tertiary,
                ObsidianColors.tertiary,
              ),
              const SizedBox(width: 8),
              _buildCategoryChip(
                'COMMITTED',
                committedCount,
                ObsidianColors.secondary,
                ObsidianColors.secondary,
              ),
              const SizedBox(width: 8),
              _buildCategoryChip(
                'WARN',
                warnCount,
                ObsidianColors.warning,
                ObsidianColors.warning,
              ),
              const SizedBox(width: 8),
              _buildCategoryChip(
                'ERROR',
                errorCount,
                ObsidianColors.error,
                ObsidianColors.error,
              ),

              const SizedBox(width: 16),
              // Search input
              Expanded(
                child: Container(
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
                        Icons.filter_alt_rounded,
                        size: 16,
                        color: ObsidianColors.outline,
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: TextField(
                          onChanged: onSearchChanged,
                          style: ObsidianTypography.bodySm(
                            color: ObsidianColors.onSurface,
                          ),
                          decoration: InputDecoration(
                            hintText:
                                'Filter message, hash, rule ID, indexer...',
                            hintStyle: ObsidianTypography.bodySm(
                              color: ObsidianColors.outline,
                            ),
                            border: InputBorder.none,
                            isDense: true,
                            contentPadding: EdgeInsets.zero,
                          ),
                        ),
                      ),
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 5,
                          vertical: 1.5,
                        ),
                        decoration: BoxDecoration(
                          color: ObsidianColors.surfaceContainerHighest,
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          'Ctrl+F',
                          style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 10),

              // Series Scope Dropdown
              Container(
                height: 36,
                padding: const EdgeInsets.symmetric(horizontal: 10),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLowest,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: DropdownButtonHideUnderline(
                  child: Builder(
                    builder: (context) {
                      final state = context.watch<StateProvider>();
                      final showTitles = state.shows
                          .map((s) => s.title)
                          .toList();
                      final currentVal =
                          (showTitles.contains(selectedShow) ||
                              selectedShow == 'All Monitored Shows')
                          ? selectedShow
                          : 'All Monitored Shows';
                      return DropdownButton<String>(
                        value: currentVal,
                        dropdownColor: ObsidianColors.surfaceContainerHigh,
                        icon: const Icon(
                          Icons.expand_more_rounded,
                          size: 16,
                          color: ObsidianColors.outline,
                        ),
                        style: ObsidianTypography.bodySm(
                          color: ObsidianColors.onSurface,
                        ),
                        items: [
                          const DropdownMenuItem(
                            value: 'All Monitored Shows',
                            child: Text('All Monitored Shows'),
                          ),
                          ...showTitles.map(
                            (t) => DropdownMenuItem(value: t, child: Text(t)),
                          ),
                        ],
                        onChanged: (val) {
                          if (val != null) {
                            onSelectShow(val);
                          }
                        },
                      );
                    },
                  ),
                ),
              ),
              const SizedBox(width: 8),

              // Time Range Dropdown
              Container(
                height: 36,
                padding: const EdgeInsets.symmetric(horizontal: 10),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLowest,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: DropdownButtonHideUnderline(
                  child: DropdownButton<String>(
                    value: selectedTimeRange,
                    dropdownColor: ObsidianColors.surfaceContainerHigh,
                    icon: const Icon(
                      Icons.schedule_rounded,
                      size: 16,
                      color: ObsidianColors.outline,
                    ),
                    style: ObsidianTypography.bodySm(
                      color: ObsidianColors.onSurface,
                    ),
                    items: const [
                      DropdownMenuItem(
                        value: 'Last 7 Days',
                        child: Text('Last 7 Days'),
                      ),
                      DropdownMenuItem(value: 'Today', child: Text('Today')),
                      DropdownMenuItem(
                        value: 'Yesterday',
                        child: Text('Yesterday'),
                      ),
                      DropdownMenuItem(
                        value: 'Last 30 Days',
                        child: Text('Last 30 Days'),
                      ),
                    ],
                    onChanged: (val) {
                      if (val != null) {
                        onSelectTimeRange(val);
                      }
                    },
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildCategoryChip(
    String label,
    int count,
    Color color,
    Color? dotColor,
  ) {
    final isSelected = selectedLevel == label;
    return InkWell(
      onTap: () => onSelectLevel(label),
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: isSelected
              ? (label == 'ALL'
                    ? ObsidianColors.primaryContainer
                    : color.withOpacity(0.18))
              : ObsidianColors.surfaceContainer,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isSelected ? color.withOpacity(0.5) : ObsidianColors.border,
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (dotColor != null) ...[
              Container(
                width: 6,
                height: 6,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: dotColor,
                ),
              ),
              const SizedBox(width: 6),
            ],
            Text(
              label,
              style: ObsidianTypography.monoCode(
                color: isSelected
                    ? (label == 'ALL'
                          ? ObsidianColors.onPrimaryContainer
                          : color)
                    : ObsidianColors.onSurfaceVariant,
              ).copyWith(fontWeight: FontWeight.bold),
            ),
            const SizedBox(width: 6),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLowest.withOpacity(0.6),
                borderRadius: BorderRadius.circular(10),
              ),
              child: Text(
                '$count',
                style: ObsidianTypography.labelBadge(
                  color: isSelected
                      ? (label == 'ALL'
                            ? ObsidianColors.onPrimaryContainer
                            : color)
                      : ObsidianColors.outline,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
