import 'package:flutter/material.dart';

import '../models/media_item.dart';
import '../state/dramas_cubit.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/cyber_badge.dart';

class SeriesDetailPage extends StatelessWidget {
  final MediaListItem item;
  final DramasCubit cubit;

  const SeriesDetailPage({super.key, required this.item, required this.cubit});

  @override
  Widget build(BuildContext context) {
    final state = cubit.state;
    final episodes = state.episodes;

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
                tooltip: 'Back to Dramas & Series',
                onPressed: () => cubit.closeDramaDetail(),
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
                tooltip: 'Refresh episodes',
                onPressed: state.isLoadingEpisodes
                    ? null
                    : () => cubit.loadEpisodes(item.id),
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
                                    Icons.tv_outlined,
                                    size: 32,
                                    color: ObsidianColors.outline,
                                  ),
                                ),
                              )
                            : Container(
                                color: ObsidianColors.surfaceContainerHigh,
                                child: const Icon(
                                  Icons.tv_outlined,
                                  size: 32,
                                  color: ObsidianColors.outline,
                                ),
                              ),
                      ),
                    ),
                    const SizedBox(width: 16),

                    // Details
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              CyberBadge.fromStatus(item.status),
                              const SizedBox(width: 10),
                              if (item.progress != null)
                                Container(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 8,
                                    vertical: 2,
                                  ),
                                  decoration: BoxDecoration(
                                    color: ObsidianColors.surfaceContainerHigh,
                                    borderRadius: BorderRadius.circular(6),
                                  ),
                                  child: Text(
                                    item.progress!.formatSummary(),
                                    style: ObsidianTypography.monoCode(
                                      color: ObsidianColors.primary,
                                      fontSize: 12,
                                    ),
                                  ),
                                ),
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
                              maxLines: 4,
                              overflow: TextOverflow.ellipsis,
                            ),
                          const SizedBox(height: 14),

                          // Action Buttons
                          Wrap(
                            spacing: 12,
                            runSpacing: 8,
                            children: [
                              ElevatedButton.icon(
                                icon: const Icon(
                                  Icons.download_for_offline,
                                  size: 18,
                                ),
                                label: const Text('Download Season 1 Pack'),
                                style: ElevatedButton.styleFrom(
                                  backgroundColor: ObsidianColors.primary,
                                  foregroundColor: ObsidianColors.onPrimary,
                                  shape: RoundedRectangleBorder(
                                    borderRadius: BorderRadius.circular(8),
                                  ),
                                ),
                                onPressed: () async {
                                  final ok = await cubit.downloadSeasonPack(
                                    item.id,
                                    season: 1,
                                  );
                                  if (context.mounted) {
                                    ScaffoldMessenger.of(context).showSnackBar(
                                      SnackBar(
                                        content: Text(
                                          ok
                                              ? 'Queued Season 1 pack for download'
                                              : 'Could not queue season pack',
                                        ),
                                        duration: const Duration(seconds: 3),
                                      ),
                                    );
                                  }
                                },
                              ),
                              OutlinedButton.icon(
                                icon: const Icon(
                                  Icons.delete_outline,
                                  size: 18,
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
                                  shape: RoundedRectangleBorder(
                                    borderRadius: BorderRadius.circular(8),
                                  ),
                                ),
                                onPressed: () async {
                                  final confirm = await showDialog<bool>(
                                    context: context,
                                    builder: (ctx) => AlertDialog(
                                      title: Text('Remove ${item.title}?'),
                                      content: const Text(
                                        'This removes the entry from your library and cancels pending jobs.',
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
                                  if (confirm != true) return;
                                  await cubit.removeDrama(item.id);
                                  cubit.closeDramaDetail();
                                },
                              ),
                              TextButton(
                                onPressed: () => cubit.closeDramaDetail(),
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

              const SizedBox(height: 20),

              // Episodes Section
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(
                    'Episodes (${episodes.length})',
                    style: ObsidianTypography.titleSm(fontSize: 16),
                  ),
                  if (state.isLoadingEpisodes)
                    const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                ],
              ),
              const SizedBox(height: 10),

              if (episodes.isEmpty && !state.isLoadingEpisodes)
                Container(
                  padding: const EdgeInsets.all(24),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Center(
                    child: Text(
                      'No episodes found for this season.',
                      style: ObsidianTypography.bodyMd(
                        color: ObsidianColors.outline,
                      ),
                    ),
                  ),
                )
              else
                ...episodes.map(
                  (ep) => _EpisodeTile(
                    ep: ep,
                    onDownload: () async {
                      final ok = await cubit.downloadEpisode(
                        item.id,
                        ep.season,
                        ep.episode,
                      );
                      if (context.mounted) {
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(
                            content: Text(
                              ok
                                  ? 'Queued Episode ${ep.episode} for download'
                                  : 'Could not queue episode',
                            ),
                            duration: const Duration(seconds: 2),
                          ),
                        );
                      }
                    },
                  ),
                ),
            ],
          ),
        ),
      ],
    );
  }
}

class _EpisodeTile extends StatelessWidget {
  final MediaEpisodeItem ep;
  final VoidCallback onDownload;

  const _EpisodeTile({required this.ep, required this.onDownload});

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: ep.isDone
              ? ObsidianColors.success.withValues(alpha: 0.3)
              : ObsidianColors.border,
        ),
      ),
      child: Row(
        children: [
          Container(
            width: 36,
            height: 36,
            decoration: BoxDecoration(
              color: ep.isDone
                  ? ObsidianColors.success.withValues(alpha: 0.15)
                  : ObsidianColors.surfaceContainerHigh,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Center(
              child: Text(
                '${ep.episode}',
                style: ObsidianTypography.monoCode(
                  fontSize: 13,
                  color: ep.isDone
                      ? ObsidianColors.success
                      : ObsidianColors.onSurface,
                ),
              ),
            ),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Episode ${ep.episode}',
                  style: ObsidianTypography.bodyMd(
                    color: ObsidianColors.onSurface,
                  ),
                ),
                if (ep.airDate != null)
                  Text(
                    'Aired: ${ep.airDate}',
                    style: ObsidianTypography.bodySm(
                      color: ObsidianColors.outline,
                    ),
                  ),
              ],
            ),
          ),
          CyberBadge.fromStatus(ep.state),
          const SizedBox(width: 12),
          if (ep.isDone)
            const Icon(
              Icons.check_circle_outline,
              color: ObsidianColors.success,
              size: 20,
            )
          else
            Tooltip(
              message: 'Download Episode ${ep.episode}',
              child: IconButton(
                icon: const Icon(Icons.download, size: 18),
                color: ObsidianColors.primary,
                onPressed: onDownload,
              ),
            ),
        ],
      ),
    );
  }
}
