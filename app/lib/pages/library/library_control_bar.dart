import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'poster_density.dart';

class LibraryControlBar extends StatelessWidget {
  final int totalCount;
  final int watchingCount;
  final int planCount;
  final int onHoldCount;
  final int completedCount;
  final int droppedCount;
  final String selectedStatus;
  final ValueChanged<String> onSelectStatus;
  final TextEditingController searchController;
  final VoidCallback onSearchChanged;
  final String selectedSort;
  final ValueChanged<String> onSelectSort;
  final PosterDensity density;
  final ValueChanged<PosterDensity> onSelectDensity;
  final VoidCallback onRescan;
  final VoidCallback onAddSeries;

  const LibraryControlBar({
    super.key,
    required this.totalCount,
    required this.watchingCount,
    required this.planCount,
    required this.onHoldCount,
    required this.completedCount,
    required this.droppedCount,
    required this.selectedStatus,
    required this.onSelectStatus,
    required this.searchController,
    required this.onSearchChanged,
    required this.selectedSort,
    required this.onSelectSort,
    required this.density,
    required this.onSelectDensity,
    required this.onRescan,
    required this.onAddSeries,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
      decoration: const BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        border: Border(bottom: BorderSide(color: ObsidianColors.border)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Wrap(
            alignment: WrapAlignment.spaceBetween,
            crossAxisAlignment: WrapCrossAlignment.center,
            spacing: 16,
            runSpacing: 10,
            children: [
              // Search Input & Filter Chips
              Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 12,
                runSpacing: 8,
                children: [
                  // Filter input box
                  Container(
                    width: 260,
                    height: 36,
                    padding: const EdgeInsets.symmetric(horizontal: 10),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerLowest,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: ObsidianColors.border),
                    ),
                    child: Row(
                      children: [
                        const Icon(Icons.filter_list_rounded,
                            size: 16, color: ObsidianColors.outline),
                        const SizedBox(width: 8),
                        Expanded(
                          child: TextField(
                            controller: searchController,
                            style: ObsidianTypography.bodySm(
                                color: ObsidianColors.onSurface),
                            decoration: InputDecoration(
                              hintText: 'Filter library... (Ctrl+F)',
                              hintStyle: ObsidianTypography.bodySm(
                                  color: ObsidianColors.outline),
                              border: InputBorder.none,
                              isDense: true,
                            ),
                            onChanged: (_) => onSearchChanged(),
                          ),
                        ),
                        Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 4, vertical: 1.5),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainerHighest,
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Text('Ctrl+F',
                              style: ObsidianTypography.monoCode(
                                  color: ObsidianColors.onSurfaceVariant,
                                  fontSize: 9)),
                        ),
                      ],
                    ),
                  ),

                  // Filter Chips
                  _buildChip('All', '$totalCount'),
                  _buildChip('Watching', '$watchingCount',
                      activeColor: ObsidianColors.tertiary),
                  _buildChip('Plan to Watch', '$planCount'),
                  _buildChip('On Hold', '$onHoldCount',
                      activeColor: ObsidianColors.warning),
                  _buildChip('Completed', '$completedCount'),
                  _buildChip('Dropped', '$droppedCount',
                      activeColor: ObsidianColors.outline),
                ],
              ),

              // Sort, Density & Actions Section
              Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 12,
                runSpacing: 8,
                children: [
                  // Sort Dropdown
                  Container(
                    height: 34,
                    padding: const EdgeInsets.symmetric(horizontal: 8),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainer,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: ObsidianColors.border),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text('SORT: ',
                            style: ObsidianTypography.labelBadge(
                                color: ObsidianColors.outline, fontSize: 9)),
                        DropdownButtonHideUnderline(
                          child: DropdownButton<String>(
                            value: selectedSort,
                            dropdownColor: ObsidianColors.surfaceContainerHigh,
                            style: ObsidianTypography.bodySm(
                                color: ObsidianColors.onSurface, fontSize: 11),
                            items: const [
                              DropdownMenuItem(
                                  value: 'updated',
                                  child: Text('Last Updated')),
                              DropdownMenuItem(
                                  value: 'title',
                                  child: Text('Title (A-Z)')),
                              DropdownMenuItem(
                                  value: 'score',
                                  child: Text('Community Score')),
                              DropdownMenuItem(
                                  value: 'progress',
                                  child: Text('Completion Progress')),
                            ],
                            onChanged: (val) {
                              if (val != null) {
                                onSelectSort(val);
                              }
                            },
                          ),
                        ),
                      ],
                    ),
                  ),

                  // Poster Size Toggle
                  Container(
                    height: 34,
                    padding: const EdgeInsets.all(2),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerLowest,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: ObsidianColors.border),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        _buildDensityButton('Compact', PosterDensity.compact),
                        _buildDensityButton('Normal', PosterDensity.normal),
                        _buildDensityButton('Large', PosterDensity.large),
                      ],
                    ),
                  ),

                  // Rescan & Add Series Buttons
                  OutlinedButton.icon(
                    onPressed: onRescan,
                    icon: const Icon(Icons.autorenew_rounded,
                        size: 16, color: ObsidianColors.tertiary),
                    label: Text('Rescan folders',
                        style: ObsidianTypography.bodySm(
                            color: ObsidianColors.onSurface, fontSize: 11)),
                    style: OutlinedButton.styleFrom(
                      backgroundColor: ObsidianColors.surfaceContainer,
                      side: const BorderSide(color: ObsidianColors.border),
                      padding: const EdgeInsets.symmetric(
                          horizontal: 10, vertical: 8),
                      shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(8)),
                    ),
                  ),

                  ElevatedButton.icon(
                    onPressed: onAddSeries,
                    icon: const Icon(Icons.add_rounded, size: 16),
                    label: Text('Add Series',
                        style: ObsidianTypography.titleSm(
                            color: ObsidianColors.onPrimary, fontSize: 12)),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: ObsidianColors.primary,
                      foregroundColor: ObsidianColors.onPrimary,
                      elevation: 0,
                      padding: const EdgeInsets.symmetric(
                          horizontal: 14, vertical: 8),
                      shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(8)),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildChip(String label, String count, {Color? activeColor}) {
    final isSelected = selectedStatus.toLowerCase() == label.toLowerCase();
    return InkWell(
      onTap: () => onSelectStatus(label),
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: isSelected
              ? ObsidianColors.primaryContainer
              : ObsidianColors.surfaceContainerHigh,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isSelected ? ObsidianColors.primary : ObsidianColors.border,
            width: isSelected ? 1 : 0.5,
          ),
        ),
        child: Text(
          '$label ($count)',
          style: ObsidianTypography.monoCode(
            color: isSelected
                ? ObsidianColors.onPrimaryContainer
                : (activeColor ?? ObsidianColors.onSurfaceVariant),
            fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
            fontSize: 11,
          ),
        ),
      ),
    );
  }

  Widget _buildDensityButton(String label, PosterDensity d) {
    final isSelected = density == d;
    return InkWell(
      onTap: () => onSelectDensity(d),
      borderRadius: BorderRadius.circular(6),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          color: isSelected
              ? ObsidianColors.surfaceContainerHighest
              : Colors.transparent,
          borderRadius: BorderRadius.circular(6),
        ),
        child: Text(
          label,
          style: ObsidianTypography.monoCode(
            color: isSelected ? ObsidianColors.primary : ObsidianColors.outline,
            fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
            fontSize: 10,
          ),
        ),
      ),
    );
  }
}
