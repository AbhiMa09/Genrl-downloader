import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../widgets/anime_poster.dart';
import 'show_detail_utils.dart';

/// A show's seasons, each downloaded as a whole. Picking single episodes is an
/// option inside a season, closed until asked for.
class ShowDetailSeasonsTab extends StatelessWidget {
  final LibraryShow show;

  const ShowDetailSeasonsTab({super.key, required this.show});

  @override
  Widget build(BuildContext context) {
    final seasons = show.seasons.isNotEmpty
        ? show.seasons
        : [SeasonItem.fromShow(show)];
    final episodeTotal = seasons.fold<int>(
      0,
      (sum, s) => sum + s.totalEpisodes,
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
                seasons.length == 1 ? '1 SEASON' : '${seasons.length} SEASONS',
                style: ObsidianTypography.labelBadge(
                  color: ObsidianColors.outline,
                ),
              ),
              Text(
                '$episodeTotal Episodes',
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.outline,
                ),
              ),
            ],
          ),
        ),
        for (final season in seasons)
          _SeasonCard(key: ValueKey(season.malId), season: season),
      ],
    );
  }
}

class _SeasonCard extends StatefulWidget {
  final SeasonItem season;

  const _SeasonCard({super.key, required this.season});

  @override
  State<_SeasonCard> createState() => _SeasonCardState();
}

class _SeasonCardState extends State<_SeasonCard> {
  bool _showEpisodes = false;
  bool _busy = false;
  List<EpisodeItem>? _episodes;

  Future<void> _loadEpisodes() async {
    final state = context.read<StateProvider>();
    final detail = await state.bridge.getShowDetail(widget.season.malId);
    if (!mounted) return;
    setState(() => _episodes = detail?.episodes ?? const []);
  }

  void _toggleEpisodes() {
    setState(() => _showEpisodes = !_showEpisodes);
    if (_showEpisodes) _loadEpisodes();
  }

  void _notify(String message) {
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text(message)));
  }

  Future<void> _downloadSeason() async {
    setState(() => _busy = true);
    final message = await context.read<StateProvider>().downloadSeason(
      widget.season.malId,
    );
    if (!mounted) return;
    setState(() => _busy = false);
    _notify(message);
    if (_showEpisodes) _loadEpisodes();
  }

  Future<void> _downloadEpisode(int number) async {
    final message = await context.read<StateProvider>().downloadEpisode(
      widget.season.malId,
      number,
    );
    if (!mounted) return;
    _notify(message);
    _loadEpisodes();
  }

  String get _seasonLabel {
    final season = widget.season;
    if (season.mediaType == 'movie') return 'Movie';
    return 'Season ${season.season}';
  }

  @override
  Widget build(BuildContext context) {
    final season = widget.season;
    final total = season.totalEpisodes;
    final done = season.downloadedEpisodes;
    final complete = total > 0 && done >= total;
    final needsInput = season.needsInput > 0;
    final active = season.activeJobs > 0 && !needsInput;
    final progress = total > 0 ? (done / total).clamp(0.0, 1.0) : 0.0;

    final statusText = needsInput
        ? 'WAITING FOR CHOICE'
        : complete
        ? 'COMPLETE'
        : active
        ? 'IN QUEUE'
        : season.failedJobs > 0
        ? 'FAILED'
        : done > 0
        ? 'PARTIAL'
        : 'NOT DOWNLOADED';
    final statusColor = needsInput
        ? ObsidianColors.warning
        : complete
        ? ObsidianColors.success
        : season.failedJobs > 0 && !active
        ? ObsidianColors.error
        : active
        ? ObsidianColors.tertiary
        : ObsidianColors.outline;

    final isMovie = season.mediaType == 'movie';
    final buttonLabel = _busy
        ? 'Queueing...'
        : active
        ? 'In Queue'
        : complete
        ? 'Downloaded'
        : season.failedJobs > 0
        ? 'Retry $_seasonLabel'
        : 'Download $_seasonLabel';

    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              SizedBox(
                width: 54,
                child: AnimePoster(urlOrPath: season.posterUrl),
              ),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Text(
                          _seasonLabel,
                          style: ObsidianTypography.titleSm().copyWith(
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                        const SizedBox(width: 8),
                        Text(
                          statusText,
                          style: ObsidianTypography.labelBadge(
                            color: statusColor,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 2),
                    Text(
                      season.title,
                      style: ObsidianTypography.bodySm(
                        color: ObsidianColors.onSurfaceVariant,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 8),
                    ClipRRect(
                      borderRadius: BorderRadius.circular(2),
                      child: LinearProgressIndicator(
                        value: progress,
                        minHeight: 4,
                        backgroundColor: ObsidianColors.surfaceContainerHighest,
                        valueColor: const AlwaysStoppedAnimation(
                          ObsidianColors.primary,
                        ),
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      needsInput
                          ? 'Waiting for your choice'
                          : total > 0
                          ? '$done / $total episodes'
                          : 'Episode count not known yet',
                      style: ObsidianTypography.monoCode(
                        color: needsInput
                            ? ObsidianColors.warning
                            : ObsidianColors.outline,
                      ).copyWith(fontSize: 11),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 12),
              Column(
                crossAxisAlignment: CrossAxisAlignment.end,
                children: [
                  if (needsInput)
                    FilledButton.icon(
                      onPressed: () =>
                          context.read<StateProvider>().selectPage(1),
                      icon: const Icon(Icons.inbox_outlined, size: 16),
                      label: const Text('Open Inbox'),
                      style: FilledButton.styleFrom(
                        backgroundColor: ObsidianColors.warning,
                        foregroundColor: ObsidianColors.surfaceContainerLowest,
                        textStyle: ObsidianTypography.monoCode(
                          color: ObsidianColors.surfaceContainerLowest,
                        ),
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(8),
                        ),
                      ),
                    )
                  else
                    Tooltip(
                      message: 'Download Season',
                      child: FilledButton.icon(
                        onPressed: (_busy || active || complete)
                            ? null
                            : _downloadSeason,
                        icon: Icon(
                          complete
                              ? Icons.check_rounded
                              : Icons.download_rounded,
                          size: 16,
                        ),
                        label: Text(buttonLabel),
                        style: FilledButton.styleFrom(
                          backgroundColor: ObsidianColors.primaryContainer,
                          foregroundColor: ObsidianColors.onPrimaryContainer,
                          textStyle: ObsidianTypography.monoCode(
                            color: ObsidianColors.onPrimaryContainer,
                          ),
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(8),
                          ),
                        ),
                      ),
                    ),
                  if (!isMovie)
                    TextButton(
                      onPressed: _toggleEpisodes,
                      child: Text(
                        _showEpisodes ? 'Hide episodes' : 'Choose episodes',
                        style: ObsidianTypography.monoCode(
                          color: ObsidianColors.outline,
                        ).copyWith(fontSize: 11),
                      ),
                    ),
                ],
              ),
            ],
          ),
          if (_showEpisodes) ...[
            const SizedBox(height: 10),
            _buildEpisodeList(total),
          ],
        ],
      ),
    );
  }

  Widget _buildEpisodeList(int total) {
    final loaded = _episodes;
    if (loaded == null) {
      return const Padding(
        padding: EdgeInsets.all(12),
        child: Center(child: CircularProgressIndicator(strokeWidth: 2)),
      );
    }
    // Episode rows exist only once something touched them; the rest are the
    // show's remaining numbers, still downloadable one by one.
    final byNumber = {for (final ep in loaded) ep.number: ep};
    final numbers = <int>{
      for (var n = 1; n <= total; n++) n,
      ...byNumber.keys,
    }.toList()..sort();
    if (numbers.isEmpty) {
      return Text(
        'Episode count not known yet.',
        style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
      );
    }
    return Column(
      children: [
        for (final number in numbers)
          _buildEpisodeRow(number, byNumber[number]),
      ],
    );
  }

  Widget _buildEpisodeRow(int number, EpisodeItem? episode) {
    final state = episode?.state.toLowerCase() ?? 'missing';
    final isDone = state == 'done';
    final inProgress = state == 'queued' || state == 'downloading';
    final file = episode?.filePath;

    return Container(
      margin: const EdgeInsets.only(bottom: 4),
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 2),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(6),
      ),
      child: Row(
        children: [
          SizedBox(
            width: 56,
            child: Text(
              'EP $number',
              style: ObsidianTypography.monoCode(
                color: ObsidianColors.onSurface,
              ),
            ),
          ),
          Expanded(
            child: Text(
              state.toUpperCase(),
              style: ObsidianTypography.labelBadge(
                color: isDone ? ObsidianColors.success : ObsidianColors.outline,
              ),
            ),
          ),
          if (isDone && file != null && file.isNotEmpty)
            TextButton(
              onPressed: () => openFileOrFolder(file),
              child: const Text('Open'),
            )
          else if (!isDone && !inProgress)
            TextButton(
              onPressed: () => _downloadEpisode(number),
              child: const Text('Download'),
            ),
        ],
      ),
    );
  }
}
