import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/library_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/anime_poster.dart';

class LibraryPage extends StatefulWidget {
  const LibraryPage({super.key});

  @override
  State<LibraryPage> createState() => _LibraryPageState();
}

class _LibraryPageState extends State<LibraryPage> {
  String _selectedStatus = 'All';

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    final filteredShows = state.shows.where((show) {
      if (_selectedStatus != 'All' && show.status != _selectedStatus) {
        return false;
      }
      if (state.searchFilter.isNotEmpty) {
        final q = state.searchFilter.toLowerCase();
        final matches = show.title.toLowerCase().contains(q) ||
            (show.englishTitle?.toLowerCase().contains(q) ?? false);
        if (!matches) return false;
      }
      return true;
    }).toList();

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Filter Tabs & Library Header Actions
          Row(
            children: [
              _FilterChip(
                label: 'All (${state.shows.length})',
                isSelected: _selectedStatus == 'All',
                onTap: () => setState(() => _selectedStatus = 'All'),
              ),
              const SizedBox(width: 8),
              _FilterChip(
                label: 'Watching',
                isSelected: _selectedStatus == 'Watching',
                onTap: () => setState(() => _selectedStatus = 'Watching'),
              ),
              const SizedBox(width: 8),
              _FilterChip(
                label: 'Completed',
                isSelected: _selectedStatus == 'Completed',
                onTap: () => setState(() => _selectedStatus = 'Completed'),
              ),
              const Spacer(),
              ElevatedButton.icon(
                onPressed: () => state.manualDiskScan(),
                icon: const Icon(Icons.refresh_rounded, size: 16),
                label: const Text('Rescan Library Folder'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainerHigh,
                  foregroundColor: ObsidianColors.onSurface,
                  elevation: 0,
                  padding:
                      const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                    side: const BorderSide(color: ObsidianColors.border),
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 20),

          // Grid of Anime Posters (2:3 aspect ratio)
          GridView.builder(
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(
              maxCrossAxisExtent: 220,
              crossAxisSpacing: 16,
              mainAxisSpacing: 20,
              childAspectRatio: 0.58,
            ),
            itemCount: filteredShows.length,
            itemBuilder: (context, index) {
              final show = filteredShows[index];
              return _ShowCard(
                show: show,
                onTap: () => state.openShowDetail(show),
              );
            },
          ),
        ],
      ),
    );
  }
}

class _FilterChip extends StatelessWidget {
  final String label;
  final bool isSelected;
  final VoidCallback onTap;

  const _FilterChip({
    required this.label,
    required this.isSelected,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        decoration: BoxDecoration(
          color: isSelected
              ? ObsidianColors.surfaceContainerHigh
              : ObsidianColors.surfaceContainerLowest,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isSelected
                ? ObsidianColors.primary.withOpacity(0.3)
                : ObsidianColors.border,
          ),
        ),
        child: Text(
          label,
          style: isSelected
              ? ObsidianTypography.bodySm(color: ObsidianColors.primary)
                  .copyWith(fontWeight: FontWeight.w600)
              : ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
        ),
      ),
    );
  }
}

class _ShowCard extends StatelessWidget {
  final LibraryShow show;
  final VoidCallback onTap;

  const _ShowCard({
    required this.show,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Poster Container (2:3)
          Expanded(
            child: Stack(
              fit: StackFit.expand,
              children: [
                AnimePoster(
                  urlOrPath: show.posterUrl,
                  aspectRatio: 2 / 3,
                  borderRadius: BorderRadius.circular(10),
                ),
                  // Top Status Badge
                  Positioned(
                    top: 8,
                    left: 8,
                    child: Container(
                      padding: const EdgeInsets.symmetric(
                          horizontal: 6, vertical: 2),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerLowest
                            .withOpacity(0.85),
                        borderRadius: BorderRadius.circular(4),
                        border: Border.all(
                            color: ObsidianColors.border, width: 0.5),
                      ),
                      child: Text(
                        show.status.toUpperCase(),
                        style: ObsidianTypography.labelBadge(
                          color: show.status == 'Completed'
                              ? ObsidianColors.success
                              : ObsidianColors.primary,
                        ),
                      ),
                    ),
                  ),

                  // Bottom Progress Bar
                  Positioned(
                    bottom: 0,
                    left: 0,
                    right: 0,
                    child: LinearProgressIndicator(
                      value: show.downloadPercentage,
                      backgroundColor:
                          ObsidianColors.surfaceContainerLowest.withOpacity(0.7),
                      valueColor: AlwaysStoppedAnimation<Color>(
                        show.downloadPercentage >= 1.0
                            ? ObsidianColors.success
                            : ObsidianColors.primary,
                      ),
                      minHeight: 4,
                    ),
                  ),
                ],
              ),
            ),
          const SizedBox(height: 8),

          // Titles and stats
          Text(
            show.title,
            style: ObsidianTypography.titleSm(),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          const SizedBox(height: 2),
          Text(
            '${show.downloadedEpisodes}/${show.totalEpisodes} Ep • ★ ${show.score}',
            style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
          ),
        ],
      ),
    );
  }
}
