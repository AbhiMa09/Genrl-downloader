import 'package:flutter/material.dart';

import '../models/media_item.dart';
import '../state/movies_cubit.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/cyber_badge.dart';

class MovieDetailPage extends StatelessWidget {
  final MediaListItem item;
  final MoviesCubit cubit;

  const MovieDetailPage({super.key, required this.item, required this.cubit});

  @override
  Widget build(BuildContext context) {
    final state = cubit.state;

    return Column(
      children: [
        // Top Sub-Header with Back Button
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          decoration: const BoxDecoration(
            color: ObsidianColors.surfaceContainerLow,
            border: Border(
              bottom: BorderSide(color: ObsidianColors.border, width: 1),
            ),
          ),
          child: Row(
            children: [
              IconButton(
                icon: const Icon(Icons.arrow_back, size: 20),
                tooltip: 'Back to Movies',
                onPressed: () => cubit.closeMovieDetail(),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  item.year != null
                      ? '${item.title} (${item.year})'
                      : item.title,
                  style: ObsidianTypography.titleSm(fontSize: 16),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              IconButton(
                icon: const Icon(Icons.refresh, size: 20),
                tooltip: 'Refresh library',
                onPressed: () => cubit.loadMovies(),
              ),
            ],
          ),
        ),

        // Main Content Area
        Expanded(
          child: ListView(
            padding: const EdgeInsets.all(20),
            children: [
              // Hero Section Card
              Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // Poster
                    ClipRRect(
                      borderRadius: BorderRadius.circular(8),
                      child: SizedBox(
                        width: 100,
                        height: 150,
                        child: item.posterUrl != null
                            ? Image.network(
                                item.posterUrl!,
                                fit: BoxFit.cover,
                                cacheWidth: 200,
                                errorBuilder: (_, __, ___) => Container(
                                  color: ObsidianColors.surfaceContainerHigh,
                                  child: const Icon(
                                    Icons.movie_outlined,
                                    size: 32,
                                    color: ObsidianColors.outline,
                                  ),
                                ),
                              )
                            : Container(
                                color: ObsidianColors.surfaceContainerHigh,
                                child: const Icon(
                                  Icons.movie_outlined,
                                  size: 32,
                                  color: ObsidianColors.outline,
                                ),
                              ),
                      ),
                    ),
                    const SizedBox(width: 20),

                    // Details Column
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              CyberBadge.fromStatus(item.status),
                              if (item.year != null) ...[
                                const SizedBox(width: 8),
                                Text(
                                  '${item.year}',
                                  style: ObsidianTypography.bodySm(
                                    color: ObsidianColors.outline,
                                  ),
                                ),
                              ],
                            ],
                          ),
                          const SizedBox(height: 10),
                          if (item.overview != null &&
                              item.overview!.isNotEmpty)
                            Text(
                              item.overview!,
                              style: ObsidianTypography.bodySm(
                                color: ObsidianColors.onSurfaceVariant,
                              ),
                              maxLines: 5,
                              overflow: TextOverflow.ellipsis,
                            ),
                          const SizedBox(height: 16),

                          // Action Buttons
                          Wrap(
                            spacing: 12,
                            runSpacing: 8,
                            children: [
                              ElevatedButton.icon(
                                icon: state.isDownloading
                                    ? const SizedBox(
                                        width: 14,
                                        height: 14,
                                        child: CircularProgressIndicator(
                                          strokeWidth: 2,
                                          color: ObsidianColors.onPrimary,
                                        ),
                                      )
                                    : const Icon(Icons.download, size: 16),
                                label: Text(
                                  item.status == 'downloaded' ||
                                          item.status == 'in_library'
                                      ? 'Re-download Movie'
                                      : 'Download Movie',
                                ),
                                style: ElevatedButton.styleFrom(
                                  backgroundColor: ObsidianColors.primary,
                                  foregroundColor: ObsidianColors.onPrimary,
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 16,
                                    vertical: 10,
                                  ),
                                  shape: RoundedRectangleBorder(
                                    borderRadius: BorderRadius.circular(8),
                                  ),
                                ),
                                onPressed: state.isDownloading
                                    ? null
                                    : () async {
                                        final ok = await cubit.downloadMovie(
                                          item.id,
                                        );
                                        if (context.mounted) {
                                          ScaffoldMessenger.of(
                                            context,
                                          ).showSnackBar(
                                            SnackBar(
                                              content: Text(
                                                ok
                                                    ? 'Download job queued for "${item.title}"'
                                                    : 'Failed to queue download for "${item.title}"',
                                              ),
                                              backgroundColor: ok
                                                  ? ObsidianColors.tertiary
                                                  : ObsidianColors.error,
                                            ),
                                          );
                                        }
                                      },
                              ),
                              OutlinedButton.icon(
                                icon: const Icon(
                                  Icons.delete_outline,
                                  size: 16,
                                  color: ObsidianColors.error,
                                ),
                                label: const Text(
                                  'Remove',
                                  style: TextStyle(color: ObsidianColors.error),
                                ),
                                style: OutlinedButton.styleFrom(
                                  side: const BorderSide(
                                    color: ObsidianColors.error,
                                  ),
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 14,
                                    vertical: 10,
                                  ),
                                  shape: RoundedRectangleBorder(
                                    borderRadius: BorderRadius.circular(8),
                                  ),
                                ),
                                onPressed: () async {
                                  final confirm = await showDialog<bool>(
                                    context: context,
                                    builder: (ctx) => AlertDialog(
                                      backgroundColor:
                                          ObsidianColors.surfaceContainerHigh,
                                      title: Text('Remove ${item.title}?'),
                                      content: const Text(
                                        'This removes the entry from your library and cancels pending jobs. Downloaded files and torrent data will not be deleted.',
                                      ),
                                      actions: [
                                        TextButton(
                                          onPressed: () =>
                                              Navigator.of(ctx).pop(false),
                                          child: const Text('Cancel'),
                                        ),
                                        TextButton(
                                          onPressed: () =>
                                              Navigator.of(ctx).pop(true),
                                          child: const Text(
                                            'Remove',
                                            style: TextStyle(
                                              color: ObsidianColors.error,
                                            ),
                                          ),
                                        ),
                                      ],
                                    ),
                                  );
                                  if (confirm == true) {
                                    await cubit.removeMovie(item.id);
                                  }
                                },
                              ),
                              TextButton(
                                onPressed: () => cubit.closeMovieDetail(),
                                style: TextButton.styleFrom(
                                  foregroundColor:
                                      ObsidianColors.onSurfaceVariant,
                                ),
                                child: const Text('Close'),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),

              const SizedBox(height: 24),

              // Metadata Info Card
              Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLow,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'Movie Information',
                      style: ObsidianTypography.titleSm(fontSize: 15),
                    ),
                    const SizedBox(height: 12),
                    _buildInfoRow('Kind', 'Feature Film'),
                    const SizedBox(height: 8),
                    _buildInfoRow('Status', item.displayStatus),
                    if (item.year != null) ...[
                      const SizedBox(height: 8),
                      _buildInfoRow('Release Year', '${item.year}'),
                    ],
                    if (item.tmdbId != null) ...[
                      const SizedBox(height: 8),
                      _buildInfoRow('TMDB ID', '${item.tmdbId}'),
                    ],
                  ],
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildInfoRow(String label, String value) {
    return Row(
      children: [
        SizedBox(
          width: 120,
          child: Text(
            label,
            style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
          ),
        ),
        Text(
          value,
          style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
        ),
      ],
    );
  }
}
