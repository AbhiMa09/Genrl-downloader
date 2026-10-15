import 'package:flutter/material.dart';
import '../../models/library_item.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

enum ShowDetailTab { episodes, aliases, rules }

class ShowDetailTabBar extends StatelessWidget {
  final LibraryShow show;
  final ShowDetailTab activeTab;
  final ValueChanged<ShowDetailTab> onTabChanged;

  const ShowDetailTabBar({
    super.key,
    required this.show,
    required this.activeTab,
    required this.onTabChanged,
  });

  @override
  Widget build(BuildContext context) {
    final epCount = show.episodes.isNotEmpty ? show.episodes.length : (show.totalEpisodes > 0 ? show.totalEpisodes : 1);
    final aliasCount = show.aliases.isNotEmpty ? show.aliases.length : 4;
    final allDone = show.downloadedEpisodes >= (show.totalEpisodes > 0 ? show.totalEpisodes : 1);

    return Container(
      padding: const EdgeInsets.all(6),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
      ),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Row(
              children: [
                _buildTabButton(
                  tab: ShowDetailTab.episodes,
                  icon: Icons.movie_outlined,
                  label: 'Episodes / Files',
                  badgeText: '$epCount',
                ),
                const SizedBox(width: 4),
                _buildTabButton(
                  tab: ShowDetailTab.aliases,
                  icon: Icons.translate_rounded,
                  label: 'Aliases & Offsets',
                  badgeText: '$aliasCount',
                ),
                const SizedBox(width: 4),
                _buildTabButton(
                  tab: ShowDetailTab.rules,
                  icon: Icons.tune_rounded,
                  label: 'Custom Rules & Ingestion',
                  hasDotIndicator: true,
                ),
              ],
            ),
            const SizedBox(width: 16),

            // Right Status Pill
            Padding(
              padding: const EdgeInsets.only(right: 8),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  children: [
                    Container(
                      width: 6,
                      height: 6,
                      decoration: BoxDecoration(
                        color: allDone ? ObsidianColors.success : ObsidianColors.tertiary,
                        shape: BoxShape.circle,
                      ),
                    ),
                    const SizedBox(width: 6),
                    Text(
                      allDone ? 'All $epCount Files Present' : '${show.downloadedEpisodes} / $epCount Downloaded',
                      style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 11),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildTabButton({
    required ShowDetailTab tab,
    required IconData icon,
    required String label,
    String? badgeText,
    bool hasDotIndicator = false,
  }) {
    final isActive = activeTab == tab;
    return InkWell(
      onTap: () => onTabChanged(tab),
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        decoration: BoxDecoration(
          color: isActive ? ObsidianColors.surfaceContainerHigh : Colors.transparent,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Row(
          children: [
            Icon(
              icon,
              size: 18,
              color: isActive ? ObsidianColors.primary : ObsidianColors.onSurfaceVariant,
            ),
            const SizedBox(width: 8),
            Text(
              label,
              style: ObsidianTypography.titleSm(
                color: isActive ? ObsidianColors.primary : ObsidianColors.onSurfaceVariant,
              ).copyWith(
                fontWeight: isActive ? FontWeight.bold : FontWeight.w500,
                fontSize: 13,
              ),
            ),
            if (badgeText != null) ...[
              const SizedBox(width: 6),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: isActive ? ObsidianColors.primary.withOpacity(0.2) : ObsidianColors.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  badgeText,
                  style: ObsidianTypography.labelBadge(
                    color: isActive ? ObsidianColors.primary : ObsidianColors.outline,
                  ),
                ),
              ),
            ],
            if (hasDotIndicator) ...[
              const SizedBox(width: 6),
              Container(
                width: 6,
                height: 6,
                decoration: const BoxDecoration(
                  color: ObsidianColors.tertiary,
                  shape: BoxShape.circle,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
