import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/library_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/anime_poster.dart';
import '../widgets/state_views.dart';

class LibraryPage extends StatefulWidget {
  const LibraryPage({super.key});

  @override
  State<LibraryPage> createState() => _LibraryPageState();
}

class _LibraryPageState extends State<LibraryPage> {
  String _selectedStatus = 'All';
  final _searchController = TextEditingController();

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    final isConnected = state.isMalConnected || state.isPublicProfile;

    // Filter shows by status tab and search query
    final filteredShows = state.shows.where((show) {
      if (_selectedStatus != 'All') {
        final norm = _selectedStatus.toLowerCase().replaceAll('_', ' ').replaceAll('-', ' ').trim();
        final showNorm = show.status.toLowerCase().replaceAll('_', ' ').replaceAll('-', ' ').trim();
        if (norm != showNorm) return false;
      }

      final query = _searchController.text.trim().toLowerCase();
      if (query.isNotEmpty) {
        final matches = show.title.toLowerCase().contains(query) ||
            (show.englishTitle?.toLowerCase().contains(query) ?? false);
        if (!matches) return false;
      }
      return true;
    }).toList();

    return CustomScrollView(
      slivers: [
        SliverPadding(
          padding: const EdgeInsets.fromLTRB(24, 24, 24, 16),
          sliver: SliverToBoxAdapter(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                // Header with search bar & actions
                Row(
                  children: [
                    Text('Anime Library', style: ObsidianTypography.headlineLg()),
                    const Spacer(),
                    ElevatedButton.icon(
                      onPressed: () => state.manualDiskScan(),
                      icon: const Icon(Icons.refresh_rounded, size: 16),
                      label: const Text('Rescan Library Folder'),
                      style: ElevatedButton.styleFrom(
                        backgroundColor: ObsidianColors.surfaceContainerHigh,
                        foregroundColor: ObsidianColors.onSurface,
                        elevation: 0,
                        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(8),
                          side: const BorderSide(color: ObsidianColors.border),
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 16),

                // Search Bar & Filter Chips
                Row(
                  children: [
                    Expanded(
                      child: Container(
                        height: 40,
                        decoration: BoxDecoration(
                          color: ObsidianColors.surfaceContainerLowest,
                          borderRadius: BorderRadius.circular(8),
                          border: Border.all(color: ObsidianColors.border),
                        ),
                        padding: const EdgeInsets.symmetric(horizontal: 12),
                        alignment: Alignment.centerLeft,
                        child: TextField(
                          controller: _searchController,
                          onChanged: (_) => setState(() {}),
                          style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
                          decoration: InputDecoration(
                            icon: const Icon(Icons.search_rounded, size: 18, color: ObsidianColors.outline),
                            hintText: 'Search anime by title, English title...',
                            hintStyle: ObsidianTypography.monoCode(color: ObsidianColors.outline),
                            border: InputBorder.none,
                            isDense: true,
                            contentPadding: EdgeInsets.zero,
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 16),

                // Status Filter Tabs
                SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    children: [
                      _FilterChip(
                        label: 'All (${state.shows.length})',
                        isSelected: _selectedStatus == 'All',
                        onTap: () => setState(() => _selectedStatus = 'All'),
                      ),
                      const SizedBox(width: 8),
                      _FilterChip(
                        label: 'Watching (${_countStatus(state.shows, "watching")})',
                        isSelected: _selectedStatus == 'Watching',
                        onTap: () => setState(() => _selectedStatus = 'Watching'),
                      ),
                      const SizedBox(width: 8),
                      _FilterChip(
                        label: 'Plan to Watch (${_countStatus(state.shows, "plan to watch")})',
                        isSelected: _selectedStatus == 'Plan to Watch',
                        onTap: () => setState(() => _selectedStatus = 'Plan to Watch'),
                      ),
                      const SizedBox(width: 8),
                      _FilterChip(
                        label: 'On Hold (${_countStatus(state.shows, "on hold")})',
                        isSelected: _selectedStatus == 'On Hold',
                        onTap: () => setState(() => _selectedStatus = 'On Hold'),
                      ),
                      const SizedBox(width: 8),
                      _FilterChip(
                        label: 'Completed (${_countStatus(state.shows, "completed")})',
                        isSelected: _selectedStatus == 'Completed',
                        onTap: () => setState(() => _selectedStatus = 'Completed'),
                      ),
                      const SizedBox(width: 8),
                      _FilterChip(
                        label: 'Dropped (${_countStatus(state.shows, "dropped")})',
                        isSelected: _selectedStatus == 'Dropped',
                        onTap: () => setState(() => _selectedStatus = 'Dropped'),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),

        // Error, Loading, or Empty state
        if (state.showsError != null)
          SliverPadding(
            padding: const EdgeInsets.symmetric(horizontal: 24),
            sliver: SliverToBoxAdapter(
              child: ErrorStateView(
                title: 'Library Sync Error',
                message: state.showsError!,
                onRetry: () => state.refreshShows(),
                retryLabel: 'Retry Library Query',
              ),
            ),
          )
        else if (state.isLoadingShows && state.shows.isEmpty)
          const SliverPadding(
            padding: EdgeInsets.symmetric(horizontal: 24),
            sliver: SliverToBoxAdapter(
              child: LoadingStateView(message: 'Querying indexed anime from core engine...'),
            ),
          )
        else if (!isConnected && state.shows.isEmpty)
          SliverPadding(
            padding: const EdgeInsets.symmetric(horizontal: 24),
            sliver: SliverToBoxAdapter(
              child: _buildNotConnectedState(state),
            ),
          )
        else if (filteredShows.isEmpty)
          const SliverPadding(
            padding: EdgeInsets.symmetric(horizontal: 24),
            sliver: SliverToBoxAdapter(
              child: EmptyStateView(
                icon: Icons.search_off_rounded,
                title: 'No Matching Anime',
                description: 'No anime found matching current status filter or search query.',
              ),
            ),
          )
        else
          // Virtualized Grid of Anime Posters (2:3 aspect ratio)
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(24, 0, 24, 24),
            sliver: SliverGrid.builder(
              gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(
                maxCrossAxisExtent: 220,
                crossAxisSpacing: 16,
                mainAxisSpacing: 20,
                childAspectRatio: 0.54,
              ),
              itemCount: filteredShows.length,
              itemBuilder: (context, index) {
                final show = filteredShows[index];
                return _ShowCard(
                  show: show,
                  onTap: () => state.openShowDetail(show),
                  onToggleAuto: () => state.toggleShowAuto(show.malId),
                );
              },
            ),
          ),
      ],
    );
  }

  int _countStatus(List<LibraryShow> shows, String status) {
    final s = status.toLowerCase().replaceAll('_', ' ').replaceAll('-', ' ').trim();
    return shows.where((show) => show.status.toLowerCase().replaceAll('_', ' ').replaceAll('-', ' ').trim() == s).length;
  }

  Widget _buildNotConnectedState(StateProvider state) {
    return Container(
      padding: const EdgeInsets.all(36),
      margin: const EdgeInsets.only(top: 20),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.link_off_rounded, size: 48, color: ObsidianColors.outline),
          const SizedBox(height: 16),
          Text('MyAnimeList Not Connected', style: ObsidianTypography.headlineMd()),
          const SizedBox(height: 8),
          ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 480),
            child: Text(
              'Connect your MAL account via OAuth2 PKCE or enter a public username in Settings to view and automate your anime watchlist.',
              textAlign: TextAlign.center,
              style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
            ),
          ),
          const SizedBox(height: 24),
          ElevatedButton.icon(
            onPressed: () {
              // Switch to settings page
              state.selectPage(4);
            },
            icon: const Icon(Icons.settings_rounded, size: 16),
            label: const Text('Connect MAL in Settings'),
            style: ElevatedButton.styleFrom(
              backgroundColor: ObsidianColors.primaryContainer,
              foregroundColor: ObsidianColors.onPrimaryContainer,
              padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
            ),
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
  final VoidCallback onToggleAuto;

  const _ShowCard({
    required this.show,
    required this.onTap,
    required this.onToggleAuto,
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
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerLowest.withOpacity(0.85),
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(color: ObsidianColors.border, width: 0.5),
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

                // Per-show Auto Toggle Button at Top Right
                Positioned(
                  top: 8,
                  right: 8,
                  child: InkWell(
                    onTap: onToggleAuto,
                    borderRadius: BorderRadius.circular(16),
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                      decoration: BoxDecoration(
                        color: show.auto
                            ? ObsidianColors.primary.withOpacity(0.9)
                            : ObsidianColors.surfaceContainerLowest.withOpacity(0.85),
                        borderRadius: BorderRadius.circular(12),
                        border: Border.all(
                          color: show.auto ? ObsidianColors.primary : ObsidianColors.border,
                          width: 0.5,
                        ),
                      ),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(
                            show.auto ? Icons.bolt_rounded : Icons.pause_circle_outline_rounded,
                            size: 12,
                            color: show.auto ? ObsidianColors.onPrimary : ObsidianColors.outline,
                          ),
                          const SizedBox(width: 3),
                          Text(
                            show.auto ? 'AUTO' : 'MANUAL',
                            style: ObsidianTypography.labelBadge(
                              color: show.auto ? ObsidianColors.onPrimary : ObsidianColors.outline,
                            ),
                          ),
                        ],
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
                    backgroundColor: ObsidianColors.surfaceContainerLowest.withOpacity(0.7),
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
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Flexible(
                child: Text(
                  '${show.watchedEpisodes}/${show.totalEpisodes} Watched',
                  style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 4),
              Text(
                '★ ${show.score}',
                style: ObsidianTypography.monoCode(color: ObsidianColors.primary),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
