import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models/media_item.dart';
import '../services/state_provider.dart';
import '../state/dramas_cubit.dart';
import '../state/indexer_cubit.dart';
import '../state/movies_cubit.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import 'loading/inline_spinner.dart';

class MediaAddDialog extends StatefulWidget {
  final String kind; // 'movie' or 'series'

  const MediaAddDialog({super.key, required this.kind});

  @override
  State<MediaAddDialog> createState() => _MediaAddDialogState();
}

class _MediaAddDialogState extends State<MediaAddDialog> {
  final _searchController = TextEditingController();
  int? _addingTmdbId;

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  void _triggerSearch() {
    final query = _searchController.text.trim();
    if (widget.kind == 'movie') {
      context.read<MoviesCubit>().search(query);
    } else {
      context.read<DramasCubit>().search(query);
    }
  }

  Future<void> _addMedia(MediaSearchResult item) async {
    setState(() => _addingTmdbId = item.tmdbId);
    bool ok = false;
    if (widget.kind == 'movie') {
      ok = await context.read<MoviesCubit>().addMovie(item.tmdbId);
    } else {
      ok = await context.read<DramasCubit>().addDrama(item.tmdbId);
    }

    if (!mounted) return;
    setState(() => _addingTmdbId = null);

    if (ok) {
      Navigator.of(context).pop();
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Added "${item.title}" to library'),
          backgroundColor: ObsidianColors.tertiary,
        ),
      );
    } else {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Failed to add "${item.title}"'),
          backgroundColor: ObsidianColors.error,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final isMovie = widget.kind == 'movie';
    final label = isMovie ? 'Movie' : 'Drama';

    final isSearching = isMovie
        ? context.watch<MoviesCubit>().state.isSearching
        : context.watch<DramasCubit>().state.isSearching;
    final searchError = isMovie
        ? context.watch<MoviesCubit>().state.searchError
        : context.watch<DramasCubit>().state.searchError;
    final results = isMovie
        ? context.watch<MoviesCubit>().state.searchResults
        : context.watch<DramasCubit>().state.searchResults;

    final isTmdbConfigured = context
        .watch<IndexerCubit>()
        .state
        .isTmdbConfigured;
    final isTmdbMissing =
        !isTmdbConfigured ||
        (searchError != null &&
            searchError.toLowerCase().contains('tmdb_not_configured'));

    return Dialog(
      backgroundColor: ObsidianColors.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(16),
        side: const BorderSide(color: ObsidianColors.border),
      ),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 640, maxHeight: 600),
        child: Padding(
          padding: const EdgeInsets.all(24.0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // Header
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Row(
                    children: [
                      Icon(
                        isMovie ? Icons.movie_outlined : Icons.tv_outlined,
                        color: ObsidianColors.primary,
                        size: 24,
                      ),
                      const SizedBox(width: 10),
                      Text(
                        'Add $label',
                        style: ObsidianTypography.headlineMd(),
                      ),
                    ],
                  ),
                  IconButton(
                    icon: const Icon(
                      Icons.close,
                      color: ObsidianColors.outline,
                    ),
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                ],
              ),
              const SizedBox(height: 16),

              // TMDB Not Configured Banner
              if (isTmdbMissing)
                Container(
                  margin: const EdgeInsets.only(bottom: 16),
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: ObsidianColors.error.withOpacity(0.12),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(
                      color: ObsidianColors.error.withOpacity(0.4),
                    ),
                  ),
                  child: Row(
                    children: [
                      const Icon(
                        Icons.warning_amber_rounded,
                        color: ObsidianColors.error,
                        size: 20,
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: Text(
                          'TMDB API key is not configured. Configure it in Settings to search.',
                          style: ObsidianTypography.bodySm(
                            color: ObsidianColors.error,
                          ),
                        ),
                      ),
                      TextButton(
                        onPressed: () {
                          Navigator.of(context).pop();
                          context.read<StateProvider>().selectPage(7);
                        },
                        child: const Text('Open Settings'),
                      ),
                    ],
                  ),
                ),

              // Search Box
              Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: _searchController,
                      style: ObsidianTypography.bodyMd(),
                      decoration: InputDecoration(
                        hintText: 'Search $label by title on TMDB...',
                        hintStyle: ObsidianTypography.bodyMd(
                          color: ObsidianColors.outline,
                        ),
                        prefixIcon: const Icon(
                          Icons.search,
                          color: ObsidianColors.outline,
                        ),
                        filled: true,
                        fillColor: ObsidianColors.surfaceContainerHigh,
                        contentPadding: const EdgeInsets.symmetric(
                          horizontal: 16,
                          vertical: 12,
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
                        focusedBorder: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(8),
                          borderSide: const BorderSide(
                            color: ObsidianColors.primary,
                          ),
                        ),
                      ),
                      onSubmitted: (_) => _triggerSearch(),
                    ),
                  ),
                  const SizedBox(width: 10),
                  ElevatedButton(
                    onPressed: isSearching ? null : _triggerSearch,
                    style: ElevatedButton.styleFrom(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 18,
                        vertical: 14,
                      ),
                      backgroundColor: ObsidianColors.primary,
                      foregroundColor: ObsidianColors.onPrimary,
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(8),
                      ),
                    ),
                    child: isSearching
                        ? const InlineSpinner(size: 16)
                        : const Text('Search'),
                  ),
                ],
              ),
              const SizedBox(height: 16),

              // Results Area
              Expanded(
                child: isSearching
                    ? const Center(child: InlineSpinner(size: 32))
                    : results.isEmpty
                    ? Center(
                        child: Text(
                          _searchController.text.isEmpty
                              ? 'Enter a title above to search'
                              : 'No results found on TMDB',
                          style: ObsidianTypography.bodyMd(
                            color: ObsidianColors.outline,
                          ),
                        ),
                      )
                    : ListView.separated(
                        itemCount: results.length,
                        separatorBuilder: (_, __) =>
                            const Divider(color: ObsidianColors.border),
                        itemBuilder: (context, index) {
                          final item = results[index];
                          final isAdding = _addingTmdbId == item.tmdbId;

                          return ListTile(
                            leading: ClipRRect(
                              borderRadius: BorderRadius.circular(4),
                              child: Container(
                                width: 36,
                                height: 54,
                                color: ObsidianColors.surfaceContainerHigh,
                                child: item.posterUrl != null
                                    ? Image.network(
                                        item.posterUrl!,
                                        fit: BoxFit.cover,
                                        errorBuilder: (_, __, ___) =>
                                            const Icon(
                                              Icons
                                                  .image_not_supported_outlined,
                                              size: 16,
                                              color: ObsidianColors.outline,
                                            ),
                                      )
                                    : const Icon(
                                        Icons.image_outlined,
                                        size: 16,
                                        color: ObsidianColors.outline,
                                      ),
                              ),
                            ),
                            title: Text(
                              item.title,
                              style: ObsidianTypography.titleSm(),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                            subtitle: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                if (item.year != null)
                                  Text(
                                    item.year.toString(),
                                    style: ObsidianTypography.bodySm(
                                      color: ObsidianColors.outline,
                                    ),
                                  ),
                                if (item.overview != null &&
                                    item.overview!.isNotEmpty)
                                  Text(
                                    item.overview!,
                                    style: ObsidianTypography.bodySm(
                                      color: ObsidianColors.onSurfaceVariant,
                                    ),
                                    maxLines: 2,
                                    overflow: TextOverflow.ellipsis,
                                  ),
                              ],
                            ),
                            trailing: ElevatedButton(
                              onPressed: isAdding
                                  ? null
                                  : () => _addMedia(item),
                              style: ElevatedButton.styleFrom(
                                backgroundColor:
                                    ObsidianColors.primaryContainer,
                                foregroundColor:
                                    ObsidianColors.onPrimaryContainer,
                                shape: RoundedRectangleBorder(
                                  borderRadius: BorderRadius.circular(6),
                                ),
                              ),
                              child: isAdding
                                  ? const InlineSpinner(size: 14)
                                  : const Text('Add'),
                            ),
                          );
                        },
                      ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
