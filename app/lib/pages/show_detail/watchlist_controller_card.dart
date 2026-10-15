import 'package:flutter/material.dart';
import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class WatchlistControllerCard extends StatelessWidget {
  final StateProvider state;
  final LibraryShow show;

  const WatchlistControllerCard({
    super.key,
    required this.state,
    required this.show,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Row(
                children: [
                  const Icon(Icons.bookmark_outline_rounded, size: 16, color: ObsidianColors.primary),
                  const SizedBox(width: 6),
                  Text('MY WATCH STATUS', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 11)),
                ],
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: (show.airingStatus.toLowerCase().contains('air') ? ObsidianColors.tertiary : ObsidianColors.secondary).withOpacity(0.18),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  show.airingStatus,
                  style: ObsidianTypography.labelBadge(
                    color: show.airingStatus.toLowerCase().contains('air') ? ObsidianColors.tertiary : ObsidianColors.secondary,
                    fontSize: 9,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 10),

          // Dropdown for Status
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: ObsidianColors.border),
            ),
            child: DropdownButtonHideUnderline(
              child: DropdownButton<String>(
                value: ['Watching', 'Plan to Watch', 'Completed', 'On Hold', 'Dropped'].contains(show.status) ? show.status : 'Watching',
                isExpanded: true,
                dropdownColor: ObsidianColors.surfaceContainerHigh,
                items: ['Watching', 'Plan to Watch', 'Completed', 'On Hold', 'Dropped'].map((s) {
                  return DropdownMenuItem(
                    value: s,
                    child: Text(s, style: ObsidianTypography.titleSm(color: ObsidianColors.onSurface, fontSize: 12)),
                  );
                }).toList(),
                onChanged: (newStatus) {
                  if (newStatus != null) {
                    state.updateShowDetails(malId: show.malId, status: newStatus);
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(
                        content: Text('Watch status updated to "$newStatus"'),
                        backgroundColor: ObsidianColors.success,
                        duration: const Duration(seconds: 2),
                      ),
                    );
                  }
                },
              ),
            ),
          ),
          const SizedBox(height: 12),

          // Episodes Watched Stepper
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text(
                  'Progress: ${show.watchedEpisodes} / ${show.totalEpisodes} eps',
                  style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 11),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 8),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  InkWell(
                    onTap: show.watchedEpisodes > 0
                        ? () => state.updateShowDetails(malId: show.malId, watchedEpisodes: show.watchedEpisodes - 1)
                        : null,
                    borderRadius: BorderRadius.circular(6),
                    child: Container(
                      padding: const EdgeInsets.all(4),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerLowest,
                        borderRadius: BorderRadius.circular(6),
                        border: Border.all(color: ObsidianColors.border),
                      ),
                      child: const Icon(Icons.remove_rounded, size: 16, color: ObsidianColors.outline),
                    ),
                  ),
                  const SizedBox(width: 8),
                  InkWell(
                    onTap: show.watchedEpisodes < show.totalEpisodes
                        ? () => state.updateShowDetails(malId: show.malId, watchedEpisodes: show.watchedEpisodes + 1)
                        : null,
                    borderRadius: BorderRadius.circular(6),
                    child: Container(
                      padding: const EdgeInsets.all(4),
                      decoration: BoxDecoration(
                        color: ObsidianColors.primary.withOpacity(0.15),
                        borderRadius: BorderRadius.circular(6),
                        border: Border.all(color: ObsidianColors.primary.withOpacity(0.4)),
                      ),
                      child: const Icon(Icons.add_rounded, size: 16, color: ObsidianColors.primary),
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
}
