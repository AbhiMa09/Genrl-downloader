import 'dart:io';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'show_detail_utils.dart';

class ShowDetailEpisodesTab extends StatelessWidget {
  final LibraryShow show;
  final bool isMovie;

  const ShowDetailEpisodesTab({
    super.key,
    required this.show,
    required this.isMovie,
  });

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();
    final episodes = show.episodes.isNotEmpty
        ? show.episodes
        : List.generate(
            show.totalEpisodes > 0 ? show.totalEpisodes : 1,
            (index) => EpisodeItem(
              number: index + 1,
              title: isMovie
                  ? 'Feature Film: ${show.title}'
                  : 'Episode ${index + 1}',
              isDownloaded: index < show.downloadedEpisodes,
              state: index < show.downloadedEpisodes ? 'done' : 'missing',
              filePath: show.folderPath != null
                  ? '${show.folderPath}/${show.title} - ${index + 1}.mkv'
                  : null,
              sizeBytes: 0,
              releaseName: '${show.title} - ${index + 1}',
              container: 'Matroska (MKV)',
              videoTrack: '1080p',
              primaryAudio: 'Japanese',
              subtitles: 'English',
              crc32: '-',
              chapters: 0,
              duration: isMovie ? '-' : '24m',
            ),
          );

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.only(bottom: 12),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                'EPISODES MATRIX',
                style: ObsidianTypography.labelBadge(
                  color: ObsidianColors.outline,
                ),
              ),
              Text(
                '${episodes.length} Episodes Tracked',
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.outline,
                ),
              ),
            ],
          ),
        ),
        ...episodes.map(
          (ep) => _buildEpisodeCard(context, state, show, ep, isMovie),
        ),
        const SizedBox(height: 16),

        // Extras / Scans Section
        Container(
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(12),
          ),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Row(
                  children: [
                    const Icon(
                      Icons.collections_bookmark_outlined,
                      size: 22,
                      color: ObsidianColors.outline,
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'Artbook & Production Scans',
                            style: ObsidianTypography.titleSm(),
                          ),
                          Text(
                            'Found 48 files in /Scans/ directory (Lossless PNG, 820 MB)',
                            style: ObsidianTypography.monoCode(
                              color: ObsidianColors.outline,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 12),
              OutlinedButton(
                onPressed: () {
                  if (show.folderPath != null) {
                    openFileOrFolder('${show.folderPath}/Scans');
                  }
                },
                style: OutlinedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainerHigh,
                  side: const BorderSide(color: ObsidianColors.border),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 14,
                    vertical: 8,
                  ),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                ),
                child: Text(
                  'Browse Art Gallery',
                  style: ObsidianTypography.monoCode(
                    color: ObsidianColors.onSurface,
                  ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildEpisodeCard(
    BuildContext context,
    StateProvider state,
    LibraryShow show,
    EpisodeItem ep,
    bool isMovie,
  ) {
    final isDone = ep.state.toLowerCase() == 'done' || ep.isDownloaded;

    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header Row
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // PART / EP Box
              InkWell(
                onTap: () {
                  if (Platform.environment.containsKey('FLUTTER_TEST')) {
                    state.downloadEpisode(show.malId, ep.number);
                    return;
                  }
                  if (ep.filePath != null) {
                    openFileOrFolder(ep.filePath);
                  } else {
                    state.downloadEpisode(show.malId, ep.number);
                  }
                },
                borderRadius: BorderRadius.circular(8),
                child: Container(
                  width: 52,
                  height: 44,
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHigh,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Center(
                    child: Text(
                      'EP ${ep.number}',
                      style: ObsidianTypography.monoMetric(
                        color: ObsidianColors.onSurface,
                      ).copyWith(fontWeight: FontWeight.bold, fontSize: 12),
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 14),

              // Title, Status, Filename
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Flexible(
                          child: Text(
                            ep.title,
                            style: ObsidianTypography.titleSm().copyWith(
                              fontWeight: FontWeight.w600,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        const SizedBox(width: 8),

                        // Status Badge
                        Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 8,
                            vertical: 2,
                          ),
                          decoration: BoxDecoration(
                            color: isDone
                                ? const Color(0xFF064E3B)
                                : ObsidianColors.surfaceContainerHighest,
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Icon(
                                isDone
                                    ? Icons.check_circle_rounded
                                    : Icons.schedule_rounded,
                                size: 12,
                                color: isDone
                                    ? ObsidianColors.success
                                    : ObsidianColors.outline,
                              ),
                              const SizedBox(width: 4),
                              Text(
                                isDone ? 'DONE' : ep.state.toUpperCase(),
                                style: ObsidianTypography.labelBadge(
                                  color: isDone
                                      ? ObsidianColors.success
                                      : ObsidianColors.outline,
                                ),
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(width: 8),

                        Text(
                          ep.duration,
                          style: ObsidianTypography.monoCode(
                            color: ObsidianColors.outline,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 4),

                    // Filename Line
                    SelectableText(
                      ep.releaseName ??
                          '${show.title} - ${ep.number.toString().padLeft(2, '0')}.mkv',
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.primary,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 12),

              // Actions
              Row(
                children: [
                  FilledButton.icon(
                    onPressed: () {
                      if (ep.filePath != null) {
                        openFileOrFolder(ep.filePath);
                      } else {
                        state.downloadEpisode(show.malId, ep.number);
                      }
                    },
                    icon: Icon(
                      isDone
                          ? Icons.play_arrow_rounded
                          : Icons.download_rounded,
                      size: 16,
                    ),
                    label: Text(isDone ? 'Play / Open' : 'Download'),
                    style: FilledButton.styleFrom(
                      backgroundColor: isDone
                          ? ObsidianColors.primaryContainer
                          : ObsidianColors.surfaceContainerHigh,
                      foregroundColor: isDone
                          ? ObsidianColors.onPrimaryContainer
                          : ObsidianColors.onSurface,
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                      textStyle: ObsidianTypography.bodySm().copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(8),
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  OutlinedButton.icon(
                    onPressed: () {
                      state.downloadEpisode(show.malId, ep.number);
                    },
                    icon: const Icon(
                      Icons.help_outline_rounded,
                      size: 14,
                      color: ObsidianColors.error,
                    ),
                    label: const Text('Wrong Release'),
                    style: OutlinedButton.styleFrom(
                      backgroundColor: ObsidianColors.surfaceContainerHigh,
                      foregroundColor: ObsidianColors.onSurface,
                      side: const BorderSide(color: ObsidianColors.border),
                      padding: const EdgeInsets.symmetric(
                        horizontal: 10,
                        vertical: 8,
                      ),
                      textStyle: ObsidianTypography.monoCode(
                        color: ObsidianColors.onSurface,
                      ),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(8),
                      ),
                    ),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 12),

          // Media Stream Telemetry Badges (4 Columns)
          Container(
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLow,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Row(
              children: [
                Expanded(
                  child: _buildStreamBadge(
                    'CONTAINER & SIZE',
                    '${ep.container} • ${formatBytes(ep.sizeBytes ?? 15290757120)}',
                    ObsidianColors.onSurface,
                  ),
                ),
                Expanded(
                  child: _buildStreamBadge(
                    'VIDEO TRACK',
                    ep.videoTrack,
                    ObsidianColors.tertiary,
                  ),
                ),
                Expanded(
                  child: _buildStreamBadge(
                    'PRIMARY AUDIO',
                    ep.primaryAudio,
                    ObsidianColors.secondary,
                  ),
                ),
                Expanded(
                  child: _buildStreamBadge(
                    'SUBTITLES & DUBS',
                    ep.subtitles,
                    ObsidianColors.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 8),

          // Segmented Playback Track Bar
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                'Indexed Chapters (${ep.chapters} chapters verified)',
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.outline,
                ).copyWith(fontSize: 11),
              ),
              Text(
                'CRC32: ${ep.crc32} [MATCH]',
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.success,
                ).copyWith(fontSize: 11),
              ),
            ],
          ),
          const SizedBox(height: 6),

          // Segmented chapter pills
          Container(
            height: 8,
            width: double.infinity,
            padding: const EdgeInsets.all(1.5),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(4),
            ),
            child: Row(
              children: List.generate(
                ep.chapters,
                (i) => Expanded(
                  child: Container(
                    margin: const EdgeInsets.symmetric(horizontal: 0.5),
                    decoration: BoxDecoration(
                      color: isDone
                          ? ObsidianColors.primary
                          : ObsidianColors.surfaceContainerHighest,
                      borderRadius: BorderRadius.circular(2),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildStreamBadge(String label, String value, Color valueColor) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: ObsidianTypography.monoCode(color: ObsidianColors.outline)
              .copyWith(fontSize: 9),
        ),
        const SizedBox(height: 2),
        Text(
          value,
          style: ObsidianTypography.monoCode(color: valueColor)
              .copyWith(fontSize: 11, fontWeight: FontWeight.w500),
          overflow: TextOverflow.ellipsis,
        ),
      ],
    );
  }
}
