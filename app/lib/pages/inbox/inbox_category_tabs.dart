import 'package:flutter/material.dart';

import '../../theme/colors.dart';
import '../../theme/typography.dart';

class InboxCategoryTabs extends StatelessWidget {
  final int pendingCount;
  final String selectedCategory;
  final ValueChanged<String> onSelectCategory;

  const InboxCategoryTabs({
    super.key,
    required this.pendingCount,
    required this.selectedCategory,
    required this.onSelectCategory,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Expanded(
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                _buildPillTab('all', 'Action Required ($pendingCount)'),
                const SizedBox(width: 6),
                _buildPillTab('matches', 'Matches (0)'),
                const SizedBox(width: 6),
                _buildPillTab('seeds', 'Seeds (0)'),
                const SizedBox(width: 6),
                _buildPillTab('files', 'Files ($pendingCount)'),
                const SizedBox(width: 6),
                _buildPillTab('batches', 'Batches (0)'),
              ],
            ),
          ),
        ),
        const SizedBox(width: 12),
        Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              'Sort: ',
              style: ObsidianTypography.monoCode(
                color: ObsidianColors.outline,
                fontSize: 11,
              ),
            ),
            Text(
              'Priority & Age',
              style: ObsidianTypography.monoCode(
                color: ObsidianColors.onSurface,
                fontWeight: FontWeight.bold,
                fontSize: 11,
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildPillTab(String key, String label) {
    final isSelected = selectedCategory == key;
    return InkWell(
      onTap: () => onSelectCategory(key),
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: isSelected
              ? ObsidianColors.surfaceContainerHighest
              : Colors.transparent,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Text(
          label,
          style: ObsidianTypography.bodySm(
            color: isSelected ? ObsidianColors.primary : ObsidianColors.outline,
            fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
            fontSize: 11,
          ),
        ),
      ),
    );
  }
}
