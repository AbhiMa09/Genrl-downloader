import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models/media_item.dart';
import '../services/state_provider.dart';
import '../state/movies_cubit.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/cyber_badge.dart';
import '../widgets/media_add_dialog.dart';
import '../widgets/poster_grid.dart';
import '../widgets/state_views.dart';

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

  void _showMovieDetail(MediaListItem item, MoviesCubit cubit) {
    showDialog<void>(
      context: context,
      builder: (dialogCtx) => _MovieDetailDialog(item: item, cubit: cubit),
    );
  }

  @override
  Widget build(BuildContext context) {
    final cubit = _watchCubit(context);
    final state = cubit.state;

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
              ElevatedButton.icon(
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
                onItemTap: (item) => _showMovieDetail(item, cubit),
              );
            },
          ),
        ),
      ],
    );
  }
}

class _MovieDetailDialog extends StatelessWidget {
  final MediaListItem item;
  final MoviesCubit cubit;

  const _MovieDetailDialog({required this.item, required this.cubit});

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      backgroundColor: ObsidianColors.surfaceContainerHigh,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      titlePadding: const EdgeInsets.fromLTRB(24, 20, 16, 12),
      contentPadding: const EdgeInsets.symmetric(horizontal: 24, vertical: 8),
      actionsAlignment: MainAxisAlignment.spaceBetween,
      actionsPadding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
      title: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Expanded(
            child: Text(
              item.year != null ? '${item.title} (${item.year})' : item.title,
              style: ObsidianTypography.titleSm(fontSize: 16),
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          IconButton(
            icon: const Icon(Icons.close, size: 20),
            onPressed: () => Navigator.of(context).pop(),
          ),
        ],
      ),
      content: SizedBox(
        width: 480,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (item.posterUrl != null)
                  ClipRRect(
                    borderRadius: BorderRadius.circular(8),
                    child: SizedBox(
                      width: 90,
                      height: 135,
                      child: Image.network(
                        item.posterUrl!,
                        fit: BoxFit.cover,
                        cacheWidth: 180,
                        errorBuilder: (_, __, ___) => Container(
                          color: ObsidianColors.surfaceContainer,
                          child: const Icon(
                            Icons.movie_outlined,
                            color: ObsidianColors.outline,
                          ),
                        ),
                      ),
                    ),
                  )
                else
                  Container(
                    width: 90,
                    height: 135,
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: const Icon(
                      Icons.movie_outlined,
                      size: 32,
                      color: ObsidianColors.outline,
                    ),
                  ),
                const SizedBox(width: 16),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Text('Status: ', style: ObsidianTypography.bodySm()),
                          CyberBadge.fromStatus(item.status),
                        ],
                      ),
                      const SizedBox(height: 8),
                      if (item.tmdbId != null)
                        Text(
                          'TMDB ID: ${item.tmdbId}',
                          style: ObsidianTypography.monoCode(
                            color: ObsidianColors.outline,
                          ),
                        ),
                      const SizedBox(height: 8),
                      if (item.overview != null && item.overview!.isNotEmpty)
                        Text(
                          item.overview!,
                          style: ObsidianTypography.bodySm(
                            color: ObsidianColors.onSurfaceVariant,
                          ),
                          maxLines: 4,
                          overflow: TextOverflow.ellipsis,
                        ),
                    ],
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
      actions: [
        TextButton.icon(
          icon: const Icon(
            Icons.delete_outline,
            size: 18,
            color: ObsidianColors.error,
          ),
          label: const Text(
            'Remove',
            style: TextStyle(color: ObsidianColors.error),
          ),
          onPressed: () async {
            final nav = Navigator.of(context);
            final confirm = await showDialog<bool>(
              context: context,
              builder: (ctx) => AlertDialog(
                title: Text('Remove ${item.title}?'),
                content: const Text(
                  'This removes the entry from your library and cancels pending jobs. Downloaded files and torrent data will not be deleted.',
                ),
                actions: [
                  TextButton(
                    onPressed: () => Navigator.of(ctx).pop(false),
                    child: const Text('Cancel'),
                  ),
                  TextButton(
                    onPressed: () => Navigator.of(ctx).pop(true),
                    child: const Text(
                      'Remove',
                      style: TextStyle(color: ObsidianColors.error),
                    ),
                  ),
                ],
              ),
            );
            if (confirm != true) return;
            await cubit.removeMovie(item.id);
            if (nav.mounted) {
              nav.pop();
            }
          },
        ),
        if (item.status == 'failed' && item.tmdbId != null)
          ElevatedButton.icon(
            icon: const Icon(Icons.refresh, size: 18),
            label: const Text('Retry'),
            style: ElevatedButton.styleFrom(
              backgroundColor: ObsidianColors.surfaceContainer,
              foregroundColor: ObsidianColors.primary,
            ),
            onPressed: () async {
              final nav = Navigator.of(context);
              await cubit.addMovie(item.tmdbId!);
              if (nav.mounted) {
                nav.pop();
              }
            },
          ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Close'),
        ),
      ],
    );
  }
}
