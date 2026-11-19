import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../services/state_provider.dart';
import '../state/movies_cubit.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/media_add_dialog.dart';
import '../widgets/poster_grid.dart';
import '../widgets/state_views.dart';
import 'movie_detail_page.dart';

class MoviesPage extends StatefulWidget {
  const MoviesPage({super.key});

  @override
  State<MoviesPage> createState() => _MoviesPageState();
}

class _MoviesPageState extends State<MoviesPage> {
  final _filterController = TextEditingController();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      _getCubit(context).loadMovies();
    });
  }

  @override
  void dispose() {
    _filterController.dispose();
    super.dispose();
  }

  MoviesCubit _getCubit(BuildContext context) {
    try {
      final cubit = Provider.of<MoviesCubit?>(context, listen: false);
      if (cubit != null) return cubit;
    } catch (_) {}
    return Provider.of<StateProvider>(context, listen: false).moviesCubit;
  }

  MoviesCubit _watchCubit(BuildContext context) {
    try {
      final cubit = Provider.of<MoviesCubit?>(context);
      if (cubit != null) return cubit;
    } catch (_) {}
    return Provider.of<StateProvider>(context).moviesCubit;
  }

  void _openAddMovieDialog() {
    showDialog<void>(
      context: context,
      builder: (_) => const MediaAddDialog(kind: 'movie'),
    );
  }

  @override
  Widget build(BuildContext context) {
    final cubit = _watchCubit(context);
    final state = cubit.state;

    if (state.selectedMovie != null) {
      return MovieDetailPage(item: state.selectedMovie!, cubit: cubit);
    }

    final filterText = _filterController.text.trim().toLowerCase();
    final items = filterText.isEmpty
        ? state.movies
        : state.movies.where((m) {
            return m.title.toLowerCase().contains(filterText) ||
                (m.year?.toString().contains(filterText) ?? false);
          }).toList();

    return Column(
      children: [
        // Toolbar
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          decoration: const BoxDecoration(
            color: ObsidianColors.surfaceContainerLow,
            border: Border(
              bottom: BorderSide(color: ObsidianColors.border, width: 1),
            ),
          ),
          child: Row(
            children: [
              Expanded(
                child: SizedBox(
                  height: 36,
                  child: TextField(
                    controller: _filterController,
                    onChanged: (_) => setState(() {}),
                    style: ObsidianTypography.bodyMd(),
                    decoration: InputDecoration(
                      hintText: 'Filter movies...',
                      hintStyle: ObsidianTypography.bodySm(
                        color: ObsidianColors.outline,
                      ),
                      prefixIcon: const Icon(
                        Icons.search,
                        size: 18,
                        color: ObsidianColors.outline,
                      ),
                      suffixIcon: _filterController.text.isNotEmpty
                          ? IconButton(
                              tooltip: 'Clear filter',
                              icon: const Icon(Icons.clear, size: 16),
                              onPressed: () {
                                _filterController.clear();
                                setState(() {});
                              },
                            )
                          : null,
                      filled: true,
                      fillColor: ObsidianColors.surfaceContainer,
                      contentPadding: const EdgeInsets.symmetric(
                        horizontal: 10,
                        vertical: 0,
                      ),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(8),
                        borderSide: const BorderSide(
                          color: ObsidianColors.border,
                        ),
                      ),
                      enabledBorder: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(8),
                        borderSide: const BorderSide(
                          color: ObsidianColors.border,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 12),
              IconButton(
                icon: const Icon(Icons.refresh, size: 20),
                tooltip: 'Refresh movies',
                onPressed: state.isLoading ? null : () => cubit.loadMovies(),
              ),
              const SizedBox(width: 8),
              Tooltip(
                message: 'Add new movie from TMDB',
                child: ElevatedButton.icon(
                  onPressed: _openAddMovieDialog,
                  icon: const Icon(Icons.add, size: 18),
                  label: const Text('Add movie'),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: ObsidianColors.primary,
                    foregroundColor: ObsidianColors.onPrimary,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),

        // Body
        Expanded(
          child: Builder(
            builder: (context) {
              if (state.isLoading && state.movies.isEmpty) {
                return const Center(
                  child: LoadingStateView(
                    message: 'Loading movies from library...',
                  ),
                );
              }
              if (state.error != null && state.movies.isEmpty) {
                return Center(
                  child: ErrorStateView(
                    title: 'Failed to load movies',
                    message: state.error!,
                    onRetry: () => cubit.loadMovies(),
                    retryLabel: 'Retry',
                  ),
                );
              }
              if (state.movies.isEmpty) {
                return Center(
                  child: EmptyStateView(
                    icon: Icons.movie_outlined,
                    title: 'No movies yet',
                    description: 'Search and add movies from TMDB to start downloading and managing your collection.',
                    action: ElevatedButton.icon(
                      onPressed: _openAddMovieDialog,
                      icon: const Icon(Icons.add),
                      label: const Text('Add movie'),
                    ),
                  ),
                );
              }
              if (items.isEmpty) {
                return Center(
                  child: Text(
                    'No movies matching "${_filterController.text}"',
                    style: ObsidianTypography.bodyMd(
                      color: ObsidianColors.outline,
                    ),
                  ),
                );
              }
              return PosterGrid(
                items: items,
                onItemTap: (item) => cubit.openMovieDetail(item),
              );
            },
          ),
        ),
      ],
    );
  }
}
