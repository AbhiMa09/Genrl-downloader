import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../widgets/state_views.dart';
import '../show_detail/show_detail_utils.dart';
import 'library_control_bar.dart';
import 'library_poster_card.dart';
import 'library_telemetry_banner.dart';
import 'poster_density.dart';

class LibraryPage extends StatefulWidget {
  const LibraryPage({super.key});

  @override
  State<LibraryPage> createState() => _LibraryPageState();
}

class _LibraryPageState extends State<LibraryPage> {
  String _selectedStatus = 'All';
  String _selectedSort = 'updated';
  PosterDensity _density = PosterDensity.normal;
  final _searchController = TextEditingController();

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    if (state.coreError != null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24.0),
          child: ErrorStateView(
            title: 'Library Sync Error',
            message: state.coreError!,
            onRetry: () => state.refreshAll(),
            retryLabel: 'Retry Library Query',
          ),
        ),
      );
    }

    if (state.shows.isEmpty && !state.isMalAuthenticated) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24.0),
          child: EmptyStateView(
            title: 'MyAnimeList Not Connected',
            description: 'Connect your MyAnimeList account to sync your plan to watch and watching lists automatically.',
            action: ElevatedButton(
              onPressed: () => state.selectPage(7),
              child: const Text('Connect MAL in Settings'),
            ),
          ),
        ),
      );
    }

    final allShows = state.shows;

    // Compute status counts matching MAL list categories
    final watchingCount = allShows.where((s) => s.status == 'Watching').length;
    final planCount = allShows.where((s) => s.status == 'Plan to Watch').length;
    final onHoldCount = allShows.where((s) => s.status == 'On Hold').length;
    final completedCount = allShows
        .where((s) => s.status == 'Completed')
        .length;
    final droppedCount = allShows.where((s) => s.status == 'Dropped').length;

    // Filter shows
    var filteredShows = allShows.where((show) {
      if (_selectedStatus != 'All') {
        if (_selectedStatus == 'Watching' && show.status != 'Watching') {
          return false;
        }
        if (_selectedStatus == 'Plan to Watch' &&
            show.status != 'Plan to Watch') {
          return false;
        }
        if (_selectedStatus == 'On Hold' && show.status != 'On Hold') {
          return false;
        }
        if (_selectedStatus == 'Completed' && show.status != 'Completed') {
          return false;
        }
        if (_selectedStatus == 'Dropped' && show.status != 'Dropped') {
          return false;
        }
      }

      final query = _searchController.text.trim().toLowerCase();
      if (query.isNotEmpty) {
        final matches =
            show.title.toLowerCase().contains(query) ||
            (show.englishTitle?.toLowerCase().contains(query) ?? false) ||
            (show.japaneseTitle?.toLowerCase().contains(query) ?? false);
        if (!matches) {
          return false;
        }
      }
      return true;
    }).toList();

    // Sort shows
    filteredShows.sort((a, b) {
      switch (_selectedSort) {
        case 'title':
          return a.title.compareTo(b.title);
        case 'score':
          return b.score.compareTo(a.score);
        case 'progress':
          final aProg = a.totalEpisodes > 0
              ? a.downloadedEpisodes / a.totalEpisodes
              : 0.0;
          final bProg = b.totalEpisodes > 0
              ? b.downloadedEpisodes / b.totalEpisodes
              : 0.0;
          return bProg.compareTo(aProg);
        case 'updated':
        default:
          return (b.malId).compareTo(a.malId);
      }
    });

    // One card per show: its seasons collapse behind the earliest one, which
    // keeps its place in the sort order above.
    final seasonsByFranchise = <int, List<LibraryShow>>{};
    for (final show in allShows) {
      final key = show.franchiseId != 0 ? show.franchiseId : show.malId;
      seasonsByFranchise.putIfAbsent(key, () => []).add(show);
    }
    final seenFranchises = <int>{};
    final cards = <LibraryShow>[];
    for (final show in filteredShows) {
      final key = show.franchiseId != 0 ? show.franchiseId : show.malId;
      if (seenFranchises.add(key)) {
        final seasons = seasonsByFranchise[key]!
          ..sort((a, b) => a.season.compareTo(b.season));
        cards.add(seasons.first);
      }
    }

    final totalTb =
        (allShows.fold<num>(0, (sum, s) => sum + s.onDiskBytes) /
                (1024 * 1024 * 1024 * 1024))
            .toStringAsFixed(1);
    final libraryPath =
        state.locationsReport?['library']?['path']?.toString() ??
        (state.libraryFolder.isNotEmpty ? state.libraryFolder : '—');

    return SingleChildScrollView(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // 1. Sub-header Telemetry Banner
          LibraryTelemetryBanner(
            seriesCount: allShows.length,
            totalTb: totalTb,
            libraryPath: libraryPath,
            state: state,
          ),

          // 2. Primary Workspace Control Bar
          LibraryControlBar(
            totalCount: allShows.length,
            watchingCount: watchingCount,
            planCount: planCount,
            onHoldCount: onHoldCount,
            completedCount: completedCount,
            droppedCount: droppedCount,
            selectedStatus: _selectedStatus,
            onSelectStatus: (status) =>
                setState(() => _selectedStatus = status),
            searchController: _searchController,
            onSearchChanged: () => setState(() {}),
            selectedSort: _selectedSort,
            onSelectSort: (sort) => setState(() => _selectedSort = sort),
            density: _density,
            onSelectDensity: (d) => setState(() => _density = d),
            onRescan: () => state.manualDiskScan(),
            onAddSeries: () => state.selectPage(4),
          ),

          // 3. Main Grid Area
          Padding(
            padding: const EdgeInsets.all(24),
            child: cards.isEmpty
                ? const EmptyStateView(
                    icon: Icons.video_library_outlined,
                    title: 'No Anime Matches Filter',
                    description: 'Try adjusting your search query or status filter chips.',
                  )
                : LayoutBuilder(
                    builder: (context, constraints) {
                      int columns;
                      switch (_density) {
                        case PosterDensity.compact:
                          columns = constraints.maxWidth > 1300
                              ? 8
                              : constraints.maxWidth > 900
                              ? 6
                              : 4;
                          break;
                        case PosterDensity.large:
                          columns = constraints.maxWidth > 1300
                              ? 4
                              : constraints.maxWidth > 900
                              ? 3
                              : 2;
                          break;
                        case PosterDensity.normal:
                          columns = constraints.maxWidth > 1300
                              ? 6
                              : constraints.maxWidth > 900
                              ? 4
                              : constraints.maxWidth > 600
                              ? 3
                              : 2;
                          break;
                      }

                      final itemWidth =
                          (constraints.maxWidth - (columns - 1) * 16) / columns;

                      // Built lazily: a list of a thousand shows must not
                      // decode a thousand posters up front.
                      return GridView.builder(
                        shrinkWrap: true,
                        physics: const NeverScrollableScrollPhysics(),
                        gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
                          crossAxisCount: columns,
                          crossAxisSpacing: 16,
                          mainAxisSpacing: 16,
                          mainAxisExtent: itemWidth * 1.5 + 90,
                        ),
                        itemCount: cards.length,
                        itemBuilder: (context, index) {
                          final show = cards[index];
                          final key = show.franchiseId != 0
                              ? show.franchiseId
                              : show.malId;
                          return LibraryPosterCard(
                            show: show,
                            seasons: seasonsByFranchise[key] ?? [show],
                            density: _density,
                            onTap: () => state.openShowDetail(show),
                            onOpenFolder: () => openFileOrFolder(
                              show.folderPath ??
                                  (libraryPath != '—' ? libraryPath : null),
                            ),
                            onToggleAuto: () =>
                                state.toggleShowAuto(show.malId),
                          );
                        },
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }
}
