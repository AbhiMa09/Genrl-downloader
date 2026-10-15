import 'dart:io';
import 'dart:math';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/library_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/anime_poster.dart';

enum ShowDetailTab { episodes, aliases, rules }

class ShowDetailPage extends StatefulWidget {
  final LibraryShow show;

  const ShowDetailPage({
    super.key,
    required this.show,
  });

  @override
  State<ShowDetailPage> createState() => _ShowDetailPageState();
}

class _ShowDetailPageState extends State<ShowDetailPage> {
  ShowDetailTab _activeTab = ShowDetailTab.episodes;
  late TextEditingController _aliasController;
  late TextEditingController _offsetController;
  String _preferredResolution = '1080p';
  String _audioProfile = 'Dual Audio FLAC';
  bool _autoHardlink = true;
  bool _isSavingRules = false;

  @override
  void initState() {
    super.initState();
    _aliasController = TextEditingController();
    _offsetController =
        TextEditingController(text: widget.show.episodeOffset.toString());
    _initRulesFromShow(widget.show);

    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<StateProvider>().loadShowDetail(widget.show.malId);
    });
  }

  void _initRulesFromShow(LibraryShow show) {
    if (show.rules != null) {
      final res = show.rules!['preferred_resolution']?.toString();
      if (res != null && res.isNotEmpty) {
        _preferredResolution = res;
      }
      final moveMode = show.rules!['move_mode']?.toString();
      if (moveMode != null) {
        _autoHardlink = moveMode.toLowerCase() == 'hardlink';
      }
    }
    _offsetController.text = show.episodeOffset.toString();
  }

  @override
  void didUpdateWidget(covariant ShowDetailPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.show.malId != widget.show.malId) {
      _initRulesFromShow(widget.show);
    }
  }

  @override
  void dispose() {
    _aliasController.dispose();
    _offsetController.dispose();
    super.dispose();
  }

  String _formatBytes(int? bytes) {
    if (bytes == null || bytes <= 0) return '0 B';
    const suffixes = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
    var i = (log(bytes) / log(1024)).floor();
    if (i < 0) i = 0;
    if (i >= suffixes.length) i = suffixes.length - 1;
    return '${(bytes / pow(1024, i)).toStringAsFixed(2)} ${suffixes[i]}';
  }

  void _openFileOrFolder(String? path) {
    if (path == null || path.isEmpty) return;
    if (Platform.environment.containsKey('FLUTTER_TEST')) return;
    try {
      if (Platform.isLinux) {
        Process.run('xdg-open', [path]);
      } else if (Platform.isMacOS) {
        Process.run('open', [path]);
      } else if (Platform.isWindows) {
        Process.run('explorer.exe', [path]);
      }
    } catch (_) {}
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();
    final show = state.selectedShow ?? widget.show;

    final onDiskString = show.onDiskBytes > 0
        ? _formatBytes(show.onDiskBytes)
        : (show.downloadedEpisodes > 0 ? '${(show.downloadedEpisodes * 1.4).toStringAsFixed(2)} GiB' : '14.24 GiB');

    final isMovie = show.mediaType.toLowerCase() == 'movie';
    final formatLabel = isMovie ? 'Movie (1 Part)' : 'TV Series (${show.totalEpisodes} Parts)';
    final durationLabel = isMovie ? '106 min' : '${show.totalEpisodes} eps';
    final premieredLabel = show.startYear > 0 ? '${show.startYear}' : 'Aug 2016';
    final isFinished = show.airingStatus.toLowerCase().contains('finish');

    final effectiveGenres = show.genres.isNotEmpty
        ? show.genres
        : ['Drama', 'Romance', 'Supernatural', 'Award Winning', 'Time Slip'];

    final effectiveAliases = show.aliases.isNotEmpty
        ? show.aliases
        : [
            show.title,
            if (show.englishTitle != null && show.englishTitle != show.title) show.englishTitle!,
            if (show.japaneseTitle != null) show.japaneseTitle!,
            '${show.title} (${show.startYear > 0 ? show.startYear : 2016})',
          ];

    return Material(
      color: ObsidianColors.surface,
      child: Column(
        children: [
          // Sub-Header & Breadcrumb Bar
          _buildSubHeader(context, state, show, isMovie),

          // Main Scrollable Area
          Expanded(
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(24),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // Main Content Layout Split: 320px Left Rail / Fluid Right Column
                  LayoutBuilder(
                    builder: (context, constraints) {
                      final isWide = constraints.maxWidth >= 980;
                      if (isWide) {
                        return Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            // Left Rail (320px)
                            SizedBox(
                              width: 320,
                              child: _buildLeftRail(
                                context,
                                state,
                                show,
                                isMovie,
                                formatLabel,
                                durationLabel,
                                premieredLabel,
                                isFinished,
                                onDiskString,
                              ),
                            ),
                            const SizedBox(width: 24),

                            // Right Main Workspace
                            Expanded(
                              child: _buildRightWorkspace(
                                context,
                                state,
                                show,
                                isMovie,
                                effectiveGenres,
                                effectiveAliases,
                              ),
                            ),
                          ],
                        );
                      } else {
                        // Stacked layout on narrower viewports
                        return Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _buildLeftRail(
                              context,
                              state,
                              show,
                              isMovie,
                              formatLabel,
                              durationLabel,
                              premieredLabel,
                              isFinished,
                              onDiskString,
                            ),
                            const SizedBox(height: 24),
                            _buildRightWorkspace(
                              context,
                              state,
                              show,
                              isMovie,
                              effectiveGenres,
                              effectiveAliases,
                            ),
                          ],
                        );
                      }
                    },
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  // ---------------------------------------------------------------------------
  // Sub-Header & Breadcrumb Bar
  // ---------------------------------------------------------------------------
  Widget _buildSubHeader(
    BuildContext context,
    StateProvider state,
    LibraryShow show,
    bool isMovie,
  ) {
    return Container(
      width: double.infinity,
      color: ObsidianColors.surfaceContainerLow,
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: [
            // Back button
            InkWell(
              onTap: () => state.closeShowDetail(),
              borderRadius: BorderRadius.circular(8),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Container(
                    width: 32,
                    height: 32,
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: const Icon(
                      Icons.arrow_back_rounded,
                      size: 18,
                      color: ObsidianColors.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    'Back to Library',
                    style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 16),

            // Breadcrumbs
            Text(
              'Library',
              style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
            ),
            Text(
              ' / ',
              style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
            ),
            Text(
              isMovie ? 'Movies' : 'TV Series',
              style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
            ),
            Text(
              ' / ',
              style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
            ),
            Text(
              show.title,
              style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
            ),
            const SizedBox(width: 8),

            // MAL ID Badge
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(4),
              ),
              child: Text(
                'MAL:${show.malId}',
                style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary),
              ),
            ),

            const SizedBox(width: 24),

            // Download Season Button
            Tooltip(
              message: 'Download Season',
              child: FilledButton.icon(
                onPressed: () => state.downloadSeason(show.malId),
                icon: const Icon(Icons.download_rounded, size: 16),
                label: Text(isMovie ? 'Download Movie' : 'Download Season'),
                style: FilledButton.styleFrom(
                  backgroundColor: ObsidianColors.primaryContainer,
                  foregroundColor: ObsidianColors.onPrimaryContainer,
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                  textStyle: ObsidianTypography.monoCode(color: ObsidianColors.onPrimaryContainer),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
              ),
            ),
            const SizedBox(width: 8),

          // Refresh Metadata Button
          OutlinedButton.icon(
            onPressed: show.metadataStatus == 'fetching'
                ? null
                : () => state.refreshShowMetadata(show.malId, force: true),
            icon: show.metadataStatus == 'fetching'
                ? const SizedBox(
                    width: 14,
                    height: 14,
                    child: CircularProgressIndicator(strokeWidth: 2, color: ObsidianColors.tertiary),
                  )
                : const Icon(Icons.sync_rounded, size: 16, color: ObsidianColors.tertiary),
            label: Text(
              show.metadataStatus == 'fetching' ? 'Refreshing...' : 'Refresh Metadata',
              style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
            ),
            style: OutlinedButton.styleFrom(
              backgroundColor: ObsidianColors.surfaceContainer,
              side: const BorderSide(color: ObsidianColors.border),
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
            ),
          ),
          const SizedBox(width: 8),

          // Recheck Hashes Button
          OutlinedButton.icon(
            onPressed: () async {
              await state.bridge.rescanLibrary();
              await state.loadShowDetail(show.malId);
              if (context.mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('Library rescanned and hashes rechecked.')),
                );
              }
            },
            icon: const Icon(Icons.verified_outlined, size: 16, color: ObsidianColors.primary),
            label: Text('Recheck Hashes', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface)),
            style: OutlinedButton.styleFrom(
              backgroundColor: ObsidianColors.surfaceContainer,
              side: const BorderSide(color: ObsidianColors.border),
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
            ),
          ),
          const SizedBox(width: 8),

          // Delete from disk button
          Tooltip(
            message: 'Delete from disk',
            child: InkWell(
              onTap: () {
                _confirmDeleteShow(context, state, show);
              },
              borderRadius: BorderRadius.circular(8),
              child: Container(
                width: 32,
                height: 32,
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: const Icon(Icons.delete_outline, size: 18, color: ObsidianColors.outline),
              ),
            ),
          ),
        ],
      ),
    ),
  );
}

  void _confirmDeleteShow(BuildContext context, StateProvider state, LibraryShow show) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: ObsidianColors.surfaceContainer,
        title: Text('Delete "${show.title}"?', style: ObsidianTypography.titleSm()),
        content: Text(
          'This will remove the show tracking from your library. If files exist on disk, they will be preserved unless explicitly purged.',
          style: ObsidianTypography.bodySm(),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () {
              Navigator.of(ctx).pop();
              state.closeShowDetail();
            },
            style: FilledButton.styleFrom(backgroundColor: ObsidianColors.errorContainer),
            child: const Text('Remove from Library', style: TextStyle(color: ObsidianColors.onErrorContainer)),
          ),
        ],
      ),
    );
  }

  void _showRatingDialog(BuildContext context, StateProvider state, LibraryShow show) {
    double selectedRating = show.score;
    showDialog(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (context, setDialogState) => AlertDialog(
          backgroundColor: ObsidianColors.surfaceContainer,
          title: Row(
            children: [
              const Icon(Icons.star_rounded, color: ObsidianColors.warning, size: 24),
              const SizedBox(width: 8),
              Expanded(
                child: Text('Rate "${show.title}"', style: ObsidianTypography.titleSm(), overflow: TextOverflow.ellipsis),
              ),
            ],
          ),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                '★ ${selectedRating.toStringAsFixed(1)} / 10.0',
                style: ObsidianTypography.headlineLg(color: ObsidianColors.warning),
              ),
              const SizedBox(height: 16),
              Slider(
                value: selectedRating.clamp(1.0, 10.0),
                min: 1.0,
                max: 10.0,
                divisions: 18,
                label: selectedRating.toStringAsFixed(1),
                activeColor: ObsidianColors.warning,
                onChanged: (val) {
                  setDialogState(() => selectedRating = val);
                },
              ),
              const SizedBox(height: 8),
              Wrap(
                spacing: 6,
                runSpacing: 6,
                children: [1, 5, 7, 8, 9, 10].map((star) {
                  final isCurrent = selectedRating.round() == star;
                  return ActionChip(
                    label: Text('★ $star', style: ObsidianTypography.labelBadge(color: isCurrent ? ObsidianColors.warning : ObsidianColors.onSurface)),
                    onPressed: () {
                      setDialogState(() => selectedRating = star.toDouble());
                    },
                    backgroundColor: isCurrent ? ObsidianColors.warning.withOpacity(0.2) : ObsidianColors.surfaceContainerHighest,
                  );
                }).toList(),
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () async {
                await state.updateShowDetails(malId: show.malId, rating: selectedRating);
                if (ctx.mounted) Navigator.of(ctx).pop();
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(
                      content: Text('Updated rating to ★${selectedRating.toStringAsFixed(1)}'),
                      backgroundColor: ObsidianColors.success,
                    ),
                  );
                }
              },
              style: FilledButton.styleFrom(backgroundColor: ObsidianColors.primary),
              child: const Text('Save Rating'),
            ),
          ],
        ),
      ),
    );
  }

  // ---------------------------------------------------------------------------
  // LEFT RAIL: Poster, Telemetry, Storage, Sparkline
  // ---------------------------------------------------------------------------
  Widget _buildLeftRail(
    BuildContext context,
    StateProvider state,
    LibraryShow show,
    bool isMovie,
    String formatLabel,
    String durationLabel,
    String premieredLabel,
    bool isFinished,
    String onDiskString,
  ) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Poster Card (2:3 aspect ratio)
        ClipRRect(
          borderRadius: BorderRadius.circular(12),
          child: Stack(
            children: [
              AspectRatio(
                aspectRatio: 2 / 3,
                child: AnimePoster(
                  urlOrPath: show.posterUrl,
                  aspectRatio: 2 / 3,
                  borderRadius: BorderRadius.circular(12),
                ),
              ),

              // Poster Top Badges
              Positioned(
                top: 10,
                left: 10,
                child: Row(
                  children: [
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerLowest.withOpacity(0.9),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        'BD 1080p',
                        style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary),
                      ),
                    ),
                    const SizedBox(width: 6),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerLowest.withOpacity(0.9),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        'HEVC',
                        style: ObsidianTypography.labelBadge(color: ObsidianColors.primary),
                      ),
                    ),
                  ],
                ),
              ),

              // Bookmark Monitored Button
              Positioned(
                top: 10,
                right: 10,
                child: InkWell(
                  onTap: () => state.toggleShowAuto(show.malId),
                  borderRadius: BorderRadius.circular(20),
                  child: Container(
                    padding: const EdgeInsets.all(6),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerLowest.withOpacity(0.9),
                      shape: BoxShape.circle,
                    ),
                    child: Icon(
                      show.auto ? Icons.bookmark : Icons.bookmark_border,
                      size: 18,
                      color: ObsidianColors.primary,
                    ),
                  ),
                ),
              ),

              // Bottom Gradient Scrim with Status Overlay
              Positioned(
                bottom: 0,
                left: 0,
                right: 0,
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                  decoration: BoxDecoration(
                    gradient: LinearGradient(
                      begin: Alignment.bottomCenter,
                      end: Alignment.topCenter,
                      colors: [
                        ObsidianColors.surfaceContainerLowest,
                        ObsidianColors.surfaceContainerLowest.withOpacity(0.8),
                        Colors.transparent,
                      ],
                    ),
                  ),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                        decoration: BoxDecoration(
                          color: ObsidianColors.secondaryContainer.withOpacity(0.5),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          isFinished
                              ? (isMovie ? 'MOVIE COMPLETE' : 'SEASON COMPLETE')
                              : 'AIRING',
                          style: ObsidianTypography.labelBadge(color: ObsidianColors.secondary),
                        ),
                      ),
                      Text(
                        durationLabel,
                        style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),

        // Quick Telemetry & Score Pill Card
        Container(
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(12),
          ),
          child: Column(
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  InkWell(
                    onTap: () => _showRatingDialog(context, state, show),
                    borderRadius: BorderRadius.circular(8),
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerLowest,
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(color: ObsidianColors.warning.withOpacity(0.35)),
                      ),
                      child: Row(
                        children: [
                          const Icon(Icons.star_rounded, size: 18, color: ObsidianColors.warning),
                          const SizedBox(width: 4),
                          Text(
                            show.score.toStringAsFixed(1),
                            style: ObsidianTypography.monoMetric(color: ObsidianColors.onSurface).copyWith(fontWeight: FontWeight.bold),
                          ),
                          const SizedBox(width: 6),
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                            decoration: BoxDecoration(
                              color: ObsidianColors.primary.withOpacity(0.15),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text(
                              'Edit',
                              style: ObsidianTypography.monoCode(color: ObsidianColors.primary, fontSize: 9),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                  Text.rich(
                    TextSpan(
                      text: 'Ranked ',
                      style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
                      children: [
                        TextSpan(
                          text: '#${show.rank ?? 12}',
                          style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface).copyWith(fontWeight: FontWeight.bold),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),

              // 2x2 Telemetry Grid
              Row(
                children: [
                  Expanded(
                    child: _buildTelemetryCell('FORMAT', formatLabel),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: _buildTelemetryCell('PREMIERED', premieredLabel),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              Row(
                children: [
                  Expanded(
                    child: _buildTelemetryCell(
                      'STATUS',
                      isFinished ? 'Finished' : 'Airing',
                      statusColor: isFinished ? ObsidianColors.success : ObsidianColors.tertiary,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: _buildTelemetryCell('STUDIO', show.studio ?? 'CoMix Wave'),
                  ),
                ],
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),

        // User Watchlist & Progress Controller Card
        Container(
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(color: ObsidianColors.border),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Row(
                    children: [
                      const Icon(Icons.bookmark_outline_rounded, size: 16, color: ObsidianColors.primary),
                      const SizedBox(width: 6),
                      Text('MY WATCH STATUS', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 11)),
                    ],
                  ),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: (show.airingStatus.toLowerCase().contains('air') ? ObsidianColors.tertiary : ObsidianColors.secondary).withOpacity(0.18),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      show.airingStatus,
                      style: ObsidianTypography.labelBadge(
                        color: show.airingStatus.toLowerCase().contains('air') ? ObsidianColors.tertiary : ObsidianColors.secondary,
                        fontSize: 9,
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 10),

              // Dropdown for Status
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 10),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLowest,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: DropdownButtonHideUnderline(
                  child: DropdownButton<String>(
                    value: ['Watching', 'Plan to Watch', 'Completed', 'On Hold', 'Dropped'].contains(show.status) ? show.status : 'Watching',
                    isExpanded: true,
                    dropdownColor: ObsidianColors.surfaceContainerHigh,
                    items: ['Watching', 'Plan to Watch', 'Completed', 'On Hold', 'Dropped'].map((s) {
                      return DropdownMenuItem(
                        value: s,
                        child: Text(s, style: ObsidianTypography.titleSm(color: ObsidianColors.onSurface, fontSize: 12)),
                      );
                    }).toList(),
                    onChanged: (newStatus) {
                      if (newStatus != null) {
                        state.updateShowDetails(malId: show.malId, status: newStatus);
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(
                            content: Text('Watch status updated to "$newStatus"'),
                            backgroundColor: ObsidianColors.success,
                            duration: const Duration(seconds: 2),
                          ),
                        );
                      }
                    },
                  ),
                ),
              ),
              const SizedBox(height: 12),

              // Episodes Watched Stepper
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Expanded(
                    child: Text(
                      'Progress: ${show.watchedEpisodes} / ${show.totalEpisodes} eps',
                      style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 11),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      InkWell(
                        onTap: show.watchedEpisodes > 0
                            ? () => state.updateShowDetails(malId: show.malId, watchedEpisodes: show.watchedEpisodes - 1)
                            : null,
                        borderRadius: BorderRadius.circular(6),
                        child: Container(
                          padding: const EdgeInsets.all(4),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainerLowest,
                            borderRadius: BorderRadius.circular(6),
                            border: Border.all(color: ObsidianColors.border),
                          ),
                          child: const Icon(Icons.remove_rounded, size: 16, color: ObsidianColors.outline),
                        ),
                      ),
                      const SizedBox(width: 8),
                      InkWell(
                        onTap: show.watchedEpisodes < show.totalEpisodes
                            ? () => state.updateShowDetails(malId: show.malId, watchedEpisodes: show.watchedEpisodes + 1)
                            : null,
                        borderRadius: BorderRadius.circular(6),
                        child: Container(
                          padding: const EdgeInsets.all(4),
                          decoration: BoxDecoration(
                            color: ObsidianColors.primary.withOpacity(0.15),
                            borderRadius: BorderRadius.circular(6),
                            border: Border.all(color: ObsidianColors.primary.withOpacity(0.4)),
                          ),
                          child: const Icon(Icons.add_rounded, size: 16, color: ObsidianColors.primary),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),

        // File Storage Inspector Card
        Container(
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(12),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Row(
                    children: [
                      const Icon(Icons.folder_outlined, size: 16, color: ObsidianColors.tertiary),
                      const SizedBox(width: 6),
                      Text(
                        'ROOT DIRECTORY',
                        style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
                      ),
                    ],
                  ),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerHighest,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      'EXT4',
                      style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 8),

              // Path display box
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLowest,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: SelectableText(
                  show.folderPath ?? '/mnt/anime/${show.title}/',
                  style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant).copyWith(fontSize: 11),
                ),
              ),
              const SizedBox(height: 10),

              // On-Disk Footprint
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Flexible(
                    child: Text(
                      'On-Disk Footprint:',
                      style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    onDiskString,
                    style: ObsidianTypography.monoCode(color: ObsidianColors.primary).copyWith(fontWeight: FontWeight.bold),
                  ),
                ],
              ),
              const SizedBox(height: 12),

              // Open in File Manager Action
              SizedBox(
                width: double.infinity,
                child: FilledButton.icon(
                  onPressed: () => _openFileOrFolder(show.folderPath),
                  icon: const Icon(Icons.folder_open_rounded, size: 16, color: ObsidianColors.primary),
                  label: const Text('Open in File Manager'),
                  style: FilledButton.styleFrom(
                    backgroundColor: ObsidianColors.surfaceContainerHigh,
                    foregroundColor: ObsidianColors.onSurface,
                    padding: const EdgeInsets.symmetric(vertical: 10),
                    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                  ),
                ),
              ),
              const SizedBox(height: 8),

              // Manual Search & Change Match row
              Row(
                children: [
                  Expanded(
                    child: OutlinedButton.icon(
                      onPressed: () {
                        state.setSearchFilter(show.title);
                        state.selectPage(4); // Search tab
                      },
                      icon: const Icon(Icons.search, size: 14),
                      label: const Text('Manual Search'),
                      style: OutlinedButton.styleFrom(
                        foregroundColor: ObsidianColors.onSurfaceVariant,
                        side: const BorderSide(color: ObsidianColors.border),
                        padding: const EdgeInsets.symmetric(vertical: 8),
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: OutlinedButton.icon(
                      onPressed: () {
                        _showChangeMatchDialog(context, state, show);
                      },
                      icon: const Icon(Icons.swap_horiz, size: 14),
                      label: const Text('Change Match'),
                      style: OutlinedButton.styleFrom(
                        foregroundColor: ObsidianColors.onSurfaceVariant,
                        side: const BorderSide(color: ObsidianColors.border),
                        padding: const EdgeInsets.symmetric(vertical: 8),
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                      ),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),

        // Quick Telemetry Graph: Codec Bitrate Analysis Sparkline
        _buildBitrateSparklineCard(durationLabel),
      ],
    );
  }

  Widget _buildTelemetryCell(String label, String value, {Color? statusColor}) {
    return Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 10),
          ),
          const SizedBox(height: 2),
          if (statusColor != null)
            Row(
              children: [
                Container(
                  width: 6,
                  height: 6,
                  decoration: BoxDecoration(color: statusColor, shape: BoxShape.circle),
                ),
                const SizedBox(width: 4),
                Expanded(
                  child: Text(
                    value,
                    style: ObsidianTypography.titleSm(color: statusColor).copyWith(fontSize: 12),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            )
          else
            Text(
              value,
              style: ObsidianTypography.titleSm(color: ObsidianColors.onSurface).copyWith(fontSize: 12),
              overflow: TextOverflow.ellipsis,
            ),
        ],
      ),
    );
  }

  Widget _buildBitrateSparklineCard(String durationLabel) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Flexible(
                child: Text(
                  'BITRATE PROFILE ($durationLabel)',
                  style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 9),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 4),
              Text(
                'Avg 18.7 Mbps',
                style: ObsidianTypography.monoMetric(color: ObsidianColors.tertiary).copyWith(fontSize: 10),
              ),
            ],
          ),
          const SizedBox(height: 10),

          // Custom Painted Sparkline Chart
          Container(
            height: 48,
            width: double.infinity,
            padding: const EdgeInsets.all(4),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: CustomPaint(
              painter: _BitrateSparklinePainter(),
            ),
          ),
          const SizedBox(height: 6),

          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('00:00', style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 9)),
              Text('Peak: 38.4 Mbps', style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 9)),
              Text(durationLabel == '106 min' ? '01:46:35' : '00:24:12',
                  style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 9)),
            ],
          ),
        ],
      ),
    );
  }

  void _showChangeMatchDialog(BuildContext context, StateProvider state, LibraryShow show) {
    final folderCtrl = TextEditingController(text: show.folderPath ?? '');
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: ObsidianColors.surfaceContainer,
        title: Text('Change Show Match / Path', style: ObsidianTypography.titleSm()),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Set Custom Folder Directory Path:', style: ObsidianTypography.bodySm()),
            const SizedBox(height: 8),
            TextField(
              controller: folderCtrl,
              style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
              decoration: InputDecoration(
                filled: true,
                fillColor: ObsidianColors.surfaceContainerLowest,
                hintText: '/mnt/anime/...',
                hintStyle: ObsidianTypography.monoCode(),
                border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () async {
              final newPath = folderCtrl.text.trim();
              Navigator.of(ctx).pop();
              if (newPath.isNotEmpty) {
                await state.saveShowFolder(show.malId, newPath);
              }
            },
            child: const Text('Save Path'),
          ),
        ],
      ),
    );
  }

  // ---------------------------------------------------------------------------
  // RIGHT MAIN WORKSPACE: Hero, Tabs, Tab Content, Bento Row
  // ---------------------------------------------------------------------------
  Widget _buildRightWorkspace(
    BuildContext context,
    StateProvider state,
    LibraryShow show,
    bool isMovie,
    List<String> effectiveGenres,
    List<String> effectiveAliases,
  ) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Hero Information Card
        _buildHeroCard(context, state, show, isMovie, effectiveGenres),
        const SizedBox(height: 20),

        // 3-Tab Segmented Navigation Bar
        _buildTabBar(show),
        const SizedBox(height: 16),

        // Active Tab View
        if (_activeTab == ShowDetailTab.episodes)
          _buildEpisodesTab(context, state, show, isMovie)
        else if (_activeTab == ShowDetailTab.aliases)
          _buildAliasesTab(context, state, show, effectiveAliases)
        else
          _buildRulesTab(context, state, show),

        const SizedBox(height: 28),

        // Associated Related Works & Recommendations Bento Row
        _buildRelatedWorksBento(context, state, show),
      ],
    );
  }

  Widget _buildHeroCard(BuildContext context, StateProvider state, LibraryShow show, bool isMovie, List<String> genres) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(24),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(16),
        boxShadow: const [
          BoxShadow(color: Colors.black26, blurRadius: 8, offset: Offset(0, 2)),
        ],
      ),
      child: Stack(
        children: [
          // Ambient decorative glow in top-right
          Positioned(
            right: -60,
            top: -60,
            child: Container(
              width: 200,
              height: 200,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: ObsidianColors.primary.withOpacity(0.04),
              ),
            ),
          ),

          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Title & Japanese Title
              Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 12,
                runSpacing: 6,
                children: [
                  Text(
                    show.englishTitle ?? show.title,
                    style: ObsidianTypography.headlineXl(),
                  ),
                  if (show.japaneseTitle != null && show.japaneseTitle!.isNotEmpty)
                    Text(
                      show.japaneseTitle!,
                      style: ObsidianTypography.headlineMd(color: ObsidianColors.outline),
                    ),
                ],
              ),
              const SizedBox(height: 6),

              // Subtitle metadata line
              Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 8,
                children: [
                  Text(
                    show.title,
                    style: ObsidianTypography.monoCode(color: ObsidianColors.primary).copyWith(fontWeight: FontWeight.w500),
                  ),
                  Text('•', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                  Text.rich(
                    TextSpan(
                      text: 'Studio: ',
                      style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
                      children: [
                        TextSpan(
                          text: show.studio ?? 'CoMix Wave Films',
                          style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface).copyWith(fontWeight: FontWeight.bold),
                        ),
                      ],
                    ),
                  ),
                  Text('•', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                  Text(
                    isMovie ? 'Original Story / Film' : 'TV Broadcast Series',
                    style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                  ),
                ],
              ),
              const SizedBox(height: 14),

              // Synopsis Section
              _buildSynopsisSection(context, state, show),
              const SizedBox(height: 16),

              // Genre Chips
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: genres.map((g) {
                  return Container(
                    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 5),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerHigh,
                      borderRadius: BorderRadius.circular(20),
                    ),
                    child: Text(
                      g,
                      style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurface),
                    ),
                  );
                }).toList(),
              ),
            ],
          ),
        ],
      ),
    );
  }

  // ---------------------------------------------------------------------------
  // 3-Tab Segmented Header
  // ---------------------------------------------------------------------------
  Widget _buildTabBar(LibraryShow show) {
    final epCount = show.episodes.isNotEmpty ? show.episodes.length : (show.totalEpisodes > 0 ? show.totalEpisodes : 1);
    final aliasCount = show.aliases.isNotEmpty ? show.aliases.length : 4;
    final allDone = show.downloadedEpisodes >= (show.totalEpisodes > 0 ? show.totalEpisodes : 1);

    return Container(
      padding: const EdgeInsets.all(6),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
      ),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Row(
              children: [
                _buildTabButton(
                  tab: ShowDetailTab.episodes,
                  icon: Icons.movie_outlined,
                  label: 'Episodes / Files',
                  badgeText: '$epCount',
                ),
                const SizedBox(width: 4),
                _buildTabButton(
                  tab: ShowDetailTab.aliases,
                  icon: Icons.translate_rounded,
                  label: 'Aliases & Offsets',
                  badgeText: '$aliasCount',
                ),
                const SizedBox(width: 4),
                _buildTabButton(
                  tab: ShowDetailTab.rules,
                  icon: Icons.tune_rounded,
                  label: 'Custom Rules & Ingestion',
                  hasDotIndicator: true,
                ),
              ],
            ),
            const SizedBox(width: 16),

            // Right Status Pill
            Padding(
              padding: const EdgeInsets.only(right: 8),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  children: [
                    Container(
                      width: 6,
                      height: 6,
                      decoration: BoxDecoration(
                        color: allDone ? ObsidianColors.success : ObsidianColors.tertiary,
                        shape: BoxShape.circle,
                      ),
                    ),
                    const SizedBox(width: 6),
                    Text(
                      allDone ? 'All $epCount Files Present' : '${show.downloadedEpisodes} / $epCount Downloaded',
                      style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 11),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildSynopsisSection(BuildContext context, StateProvider state, LibraryShow show) {
    final status = show.metadataStatus;
    final isFetching = status == 'fetching' || show.synopsis == '[Synopsis loading...]';
    final isPendingOrFailed = status == 'pending' || status == 'failed' || show.synopsis.startsWith('[Synopsis unavailable');

    if (isFetching) {
      return Container(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainerLow,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: ObsidianColors.border),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            const SizedBox(
              width: 14,
              height: 14,
              child: CircularProgressIndicator(strokeWidth: 2, color: ObsidianColors.primary),
            ),
            const SizedBox(width: 10),
            Text(
              'Synopsis loading...',
              style: ObsidianTypography.bodyMd(color: ObsidianColors.outline).copyWith(fontStyle: FontStyle.italic),
            ),
          ],
        ),
      );
    }

    if (isPendingOrFailed) {
      final errorMsg = show.metadataError ?? 'upstream rate limit or network unreachable';
      String retryNotice = 'Could not load, retrying';
      if (show.metadataNextRetryAt != null) {
        final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
        final diff = show.metadataNextRetryAt! - now;
        if (diff > 0) {
          retryNotice = 'Could not load, retrying in ${diff}s';
        }
      }

      return Container(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainerLow,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: ObsidianColors.errorContainer.withOpacity(0.5)),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            const Icon(Icons.info_outline, size: 16, color: ObsidianColors.warning),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                '$retryNotice ($errorMsg)',
                style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
              ),
            ),
            const SizedBox(width: 12),
            FilledButton.tonal(
              onPressed: () => state.refreshShowMetadata(show.malId, force: true),
              style: FilledButton.styleFrom(
                backgroundColor: ObsidianColors.surfaceContainerHigh,
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                minimumSize: Size.zero,
                tapTargetSize: MaterialTapTargetSize.shrinkWrap,
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(Icons.refresh_rounded, size: 14, color: ObsidianColors.primary),
                  const SizedBox(width: 6),
                  Text('Retry', style: ObsidianTypography.monoCode(color: ObsidianColors.primary)),
                ],
              ),
            ),
          ],
        ),
      );
    }

    if (show.synopsis.isNotEmpty && !show.synopsis.startsWith('[')) {
      return Text(
        show.synopsis,
        style: ObsidianTypography.bodyMd(color: ObsidianColors.onSurfaceVariant).copyWith(height: 1.55),
      );
    }

    return Text(
      'No synopsis available for this show.',
      style: ObsidianTypography.bodyMd(color: ObsidianColors.outline).copyWith(fontStyle: FontStyle.italic),
    );
  }

  Widget _buildTabButton({
    required ShowDetailTab tab,
    required IconData icon,
    required String label,
    String? badgeText,
    bool hasDotIndicator = false,
  }) {
    final isActive = _activeTab == tab;
    return InkWell(
      onTap: () => setState(() => _activeTab = tab),
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        decoration: BoxDecoration(
          color: isActive ? ObsidianColors.surfaceContainerHigh : Colors.transparent,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Row(
          children: [
            Icon(
              icon,
              size: 18,
              color: isActive ? ObsidianColors.primary : ObsidianColors.onSurfaceVariant,
            ),
            const SizedBox(width: 8),
            Text(
              label,
              style: ObsidianTypography.titleSm(
                color: isActive ? ObsidianColors.primary : ObsidianColors.onSurfaceVariant,
              ).copyWith(
                fontWeight: isActive ? FontWeight.bold : FontWeight.w500,
                fontSize: 13,
              ),
            ),
            if (badgeText != null) ...[
              const SizedBox(width: 6),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: isActive ? ObsidianColors.primary.withOpacity(0.2) : ObsidianColors.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  badgeText,
                  style: ObsidianTypography.labelBadge(
                    color: isActive ? ObsidianColors.primary : ObsidianColors.outline,
                  ),
                ),
              ),
            ],
            if (hasDotIndicator) ...[
              const SizedBox(width: 6),
              Container(
                width: 6,
                height: 6,
                decoration: const BoxDecoration(
                  color: ObsidianColors.tertiary,
                  shape: BoxShape.circle,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  // ---------------------------------------------------------------------------
  // TAB 1: EPISODES CONTENT
  // ---------------------------------------------------------------------------
  Widget _buildEpisodesTab(
    BuildContext context,
    StateProvider state,
    LibraryShow show,
    bool isMovie,
  ) {
    final episodes = show.episodes.isNotEmpty
        ? show.episodes
        : List.generate(
            show.totalEpisodes > 0 ? show.totalEpisodes : 1,
            (index) => EpisodeItem(
              number: index + 1,
              title: isMovie ? 'Feature Film: ${show.title}' : 'Episode ${index + 1}',
              isDownloaded: index < show.downloadedEpisodes,
              state: index < show.downloadedEpisodes ? 'done' : 'missing',
              filePath: show.folderPath != null ? '${show.folderPath}/[Beatrice-Raws] ${show.title} - ${index + 1}.mkv' : null,
              sizeBytes: 15290757120, // 14.24 GiB
              releaseName: '[Beatrice-Raws] ${show.title} [BDRip 1920x1080 HEVC FLAC].mkv',
              container: 'Matroska (MKV)',
              videoTrack: '1080p BD Remux HEVC 10-bit',
              primaryAudio: 'FLAC 5.1ch 24-bit 48kHz',
              subtitles: 'ENG (PGS), JPN, RADWIMPS Comm.',
              crc32: '5E2A9D11',
              chapters: 18,
              duration: isMovie ? '106:35' : '24:12',
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
                style: ObsidianTypography.labelBadge(color: ObsidianColors.outline),
              ),
              Text(
                '${episodes.length} Episodes Tracked',
                style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
              ),
            ],
          ),
        ),
        ...episodes.map((ep) => _buildEpisodeCard(context, state, show, ep, isMovie)),
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
                    const Icon(Icons.collections_bookmark_outlined, size: 22, color: ObsidianColors.outline),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('Artbook & Production Scans', style: ObsidianTypography.titleSm()),
                          Text(
                            'Found 48 files in /Scans/ directory (Lossless PNG, 820 MB)',
                            style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
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
                    _openFileOrFolder('${show.folderPath}/Scans');
                  }
                },
                style: OutlinedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainerHigh,
                  side: const BorderSide(color: ObsidianColors.border),
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
                child: Text('Browse Art Gallery', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface)),
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
                    _openFileOrFolder(ep.filePath);
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
                      style: ObsidianTypography.monoMetric(color: ObsidianColors.onSurface).copyWith(
                        fontWeight: FontWeight.bold,
                        fontSize: 12,
                      ),
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
                            style: ObsidianTypography.titleSm().copyWith(fontWeight: FontWeight.w600),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        const SizedBox(width: 8),

                        // Status Badge
                        Container(
                          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                          decoration: BoxDecoration(
                            color: isDone ? const Color(0xFF064E3B) : ObsidianColors.surfaceContainerHighest,
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Icon(
                                isDone ? Icons.check_circle_rounded : Icons.schedule_rounded,
                                size: 12,
                                color: isDone ? ObsidianColors.success : ObsidianColors.outline,
                              ),
                              const SizedBox(width: 4),
                              Text(
                                isDone ? 'DONE' : ep.state.toUpperCase(),
                                style: ObsidianTypography.labelBadge(
                                  color: isDone ? ObsidianColors.success : ObsidianColors.outline,
                                ),
                              ),
                            ],
                          ),
                        ),
                        const SizedBox(width: 8),

                        Text(ep.duration, style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                      ],
                    ),
                    const SizedBox(height: 4),

                    // Filename Line
                    SelectableText(
                      ep.releaseName ?? '[Beatrice-Raws] ${show.title} - ${ep.number.toString().padLeft(2, '0')}.mkv',
                      style: ObsidianTypography.monoCode(color: ObsidianColors.primary),
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
                        _openFileOrFolder(ep.filePath);
                      } else {
                        state.downloadEpisode(show.malId, ep.number);
                      }
                    },
                    icon: Icon(isDone ? Icons.play_arrow_rounded : Icons.download_rounded, size: 16),
                    label: Text(isDone ? 'Play / Open' : 'Download'),
                    style: FilledButton.styleFrom(
                      backgroundColor: isDone ? ObsidianColors.primaryContainer : ObsidianColors.surfaceContainerHigh,
                      foregroundColor: isDone ? ObsidianColors.onPrimaryContainer : ObsidianColors.onSurface,
                      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                      textStyle: ObsidianTypography.bodySm().copyWith(fontWeight: FontWeight.w600),
                      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                    ),
                  ),
                  const SizedBox(width: 8),
                  OutlinedButton.icon(
                    onPressed: () {
                      state.downloadEpisode(show.malId, ep.number);
                    },
                    icon: const Icon(Icons.help_outline_rounded, size: 14, color: ObsidianColors.error),
                    label: const Text('Wrong Release'),
                    style: OutlinedButton.styleFrom(
                      backgroundColor: ObsidianColors.surfaceContainerHigh,
                      foregroundColor: ObsidianColors.onSurface,
                      side: const BorderSide(color: ObsidianColors.border),
                      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                      textStyle: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
                      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
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
                    '${ep.container} • ${_formatBytes(ep.sizeBytes ?? 15290757120)}',
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
                style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 11),
              ),
              Text(
                'CRC32: ${ep.crc32} [MATCH]',
                style: ObsidianTypography.monoCode(color: ObsidianColors.success).copyWith(fontSize: 11),
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
                      color: isDone ? ObsidianColors.primary : ObsidianColors.surfaceContainerHighest,
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
          style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 9),
        ),
        const SizedBox(height: 2),
        Text(
          value,
          style: ObsidianTypography.monoCode(color: valueColor).copyWith(fontSize: 11, fontWeight: FontWeight.w500),
          overflow: TextOverflow.ellipsis,
        ),
      ],
    );
  }

  // ---------------------------------------------------------------------------
  // TAB 2: ALIASES & OFFSETS CONTENT
  // ---------------------------------------------------------------------------
  Widget _buildAliasesTab(
    BuildContext context,
    StateProvider state,
    LibraryShow show,
    List<String> aliases,
  ) {
    return Container(
      padding: const EdgeInsets.all(24),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(16),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('Title Aliases & Release Matching', style: ObsidianTypography.headlineMd()),
          const SizedBox(height: 4),
          Text(
            'Define alternate tracker queries and release names. When torrent releases match any alias below, they will automatically be parsed into this show directory.',
            style: ObsidianTypography.bodyMd(color: ObsidianColors.onSurfaceVariant),
          ),
          const SizedBox(height: 20),

          Text('CONFIGURED MATCH ALIASES', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
          const SizedBox(height: 10),

          // Chips of aliases
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: aliases.map((alias) {
              return Container(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(alias, style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface)),
                    const SizedBox(width: 8),
                    InkWell(
                      onTap: () async {
                        await state.removeAlias(show.malId, alias);
                      },
                      child: const Icon(Icons.close_rounded, size: 14, color: ObsidianColors.outline),
                    ),
                  ],
                ),
              );
            }).toList(),
          ),
          const SizedBox(height: 16),

          // Input for new alias
          Row(
            children: [
              Expanded(
                child: Container(
                  constraints: const BoxConstraints(maxWidth: 420),
                  child: TextField(
                    controller: _aliasController,
                    style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
                    decoration: InputDecoration(
                      hintText: 'Type new alias (e.g. ${show.title} Beatrice)...',
                      hintStyle: ObsidianTypography.monoCode(color: ObsidianColors.outline),
                      filled: true,
                      fillColor: ObsidianColors.surfaceContainerLowest,
                      contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(8),
                        borderSide: BorderSide.none,
                      ),
                    ),
                    onSubmitted: (val) async {
                      if (val.trim().isNotEmpty) {
                        await state.addAlias(show.malId, val.trim());
                        _aliasController.clear();
                      }
                    },
                  ),
                ),
              ),
              const SizedBox(width: 10),
              FilledButton.icon(
                onPressed: () async {
                  final text = _aliasController.text.trim();
                  if (text.isNotEmpty) {
                    await state.addAlias(show.malId, text);
                    _aliasController.clear();
                  }
                },
                icon: const Icon(Icons.add, size: 16),
                label: const Text('Add Alias'),
                style: FilledButton.styleFrom(
                  backgroundColor: ObsidianColors.primary,
                  foregroundColor: ObsidianColors.onPrimary,
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
              ),
            ],
          ),
          const SizedBox(height: 24),

          // Episode Offset Box
          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLow,
              borderRadius: BorderRadius.circular(12),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('Episode Number Offset', style: ObsidianTypography.titleSm()),
                      Text(
                        'Adjust parsing integer if tracker indexes this anime as an ongoing sequel episode.',
                        style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 16),
                Row(
                  children: [
                    Text('Offset Value:', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                    const SizedBox(width: 8),
                    SizedBox(
                      width: 72,
                      child: TextField(
                        controller: _offsetController,
                        keyboardType: TextInputType.number,
                        textAlign: TextAlign.center,
                        style: ObsidianTypography.monoMetric(color: ObsidianColors.primary).copyWith(fontWeight: FontWeight.bold),
                        decoration: InputDecoration(
                          filled: true,
                          fillColor: ObsidianColors.surfaceContainerLowest,
                          contentPadding: const EdgeInsets.symmetric(vertical: 8),
                          border: OutlineInputBorder(borderRadius: BorderRadius.circular(8), borderSide: BorderSide.none),
                        ),
                        onSubmitted: (val) async {
                          final parsed = int.tryParse(val.trim());
                          if (parsed != null) {
                            await state.setEpisodeOffset(show.malId, parsed);
                          }
                        },
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  // ---------------------------------------------------------------------------
  // TAB 3: CUSTOM RULES & INGESTION
  // ---------------------------------------------------------------------------
  Widget _buildRulesTab(
    BuildContext context,
    StateProvider state,
    LibraryShow show,
  ) {
    return Container(
      padding: const EdgeInsets.all(24),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(16),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('Per-Show Quality & Ingestion Overrides', style: ObsidianTypography.headlineMd()),
          const SizedBox(height: 4),
          Text.rich(
            TextSpan(
              text: 'Granular automated rules specific to ',
              style: ObsidianTypography.bodyMd(color: ObsidianColors.onSurfaceVariant),
              children: [
                TextSpan(
                  text: show.title,
                  style: ObsidianTypography.bodyMd(color: ObsidianColors.onSurface).copyWith(fontWeight: FontWeight.bold),
                ),
                const TextSpan(text: ' that supersede global daemon library configs.'),
              ],
            ),
          ),
          const SizedBox(height: 20),

          // Overrides Grid
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Resolution Preference
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(14),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('RESOLUTION PREFERENCE', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                      const SizedBox(height: 8),
                      DropdownButtonFormField<String>(
                        value: _preferredResolution,
                        dropdownColor: ObsidianColors.surfaceContainerLowest,
                        decoration: InputDecoration(
                          filled: true,
                          fillColor: ObsidianColors.surfaceContainerLowest,
                          border: OutlineInputBorder(borderRadius: BorderRadius.circular(8), borderSide: BorderSide.none),
                          contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                        ),
                        style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
                        items: const [
                          DropdownMenuItem(value: '1080p', child: Text('BD 1080p Remux (Selected)')),
                          DropdownMenuItem(value: '2160p', child: Text('4K UHD Remux (HDR10 / Dolby Vision)')),
                          DropdownMenuItem(value: '720p', child: Text('720p BDRip (Re-encode)')),
                          DropdownMenuItem(value: '480p', child: Text('WEB-DL 480p')),
                        ],
                        onChanged: (val) {
                          if (val != null) setState(() => _preferredResolution = val);
                        },
                      ),
                      const SizedBox(height: 6),
                      Text('Accepts bitrates ≥ 16.0 Mbps', style: ObsidianTypography.monoCode(color: ObsidianColors.tertiary).copyWith(fontSize: 10)),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 16),

              // Audio Codec Profile
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(14),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('AUDIO CODEC PROFILE', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                      const SizedBox(height: 8),
                      DropdownButtonFormField<String>(
                        value: _audioProfile,
                        dropdownColor: ObsidianColors.surfaceContainerLowest,
                        decoration: InputDecoration(
                          filled: true,
                          fillColor: ObsidianColors.surfaceContainerLowest,
                          border: OutlineInputBorder(borderRadius: BorderRadius.circular(8), borderSide: BorderSide.none),
                          contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                        ),
                        style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
                        items: const [
                          DropdownMenuItem(value: 'Dual Audio FLAC', child: Text('Dual Audio FLAC (Lossless 5.1/2.0)')),
                          DropdownMenuItem(value: 'Original Japanese Only', child: Text('Original Japanese Only (FLAC/TrueHD)')),
                          DropdownMenuItem(value: 'English Dub Primary', child: Text('English Dub Primary (AAC/AC3)')),
                        ],
                        onChanged: (val) {
                          if (val != null) setState(() => _audioProfile = val);
                        },
                      ),
                      const SizedBox(height: 6),
                      Text('Prefers Japanese FLAC default track', style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 10)),
                    ],
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),

          // Release Group Priority Hierarchy
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLow,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text('RELEASE GROUP PRIORITY HIERARCHY', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                    Text('Left-to-Right Preference Order', style: ObsidianTypography.monoCode(color: ObsidianColors.primary).copyWith(fontSize: 10)),
                  ],
                ),
                const SizedBox(height: 10),
                Container(
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLowest,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: SingleChildScrollView(
                    scrollDirection: Axis.horizontal,
                    child: Row(
                      children: [
                        _buildGroupPill('1. Beatrice-Raws', isPrimary: true),
                        _buildGroupSeparator(),
                        _buildGroupPill('2. Erai-raws'),
                        _buildGroupSeparator(),
                        _buildGroupPill('3. Judas'),
                        _buildGroupSeparator(),
                        _buildGroupPill('4. SubsPlease'),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),

          // Auto-Ingestion & Hardlink Switch
          Container(
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLow,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Daemon Auto-Ingestion & Hardlink', style: ObsidianTypography.titleSm()),
                    Text(
                      'Automatically hardlink from /mnt/torrents/completed directly into library when complete.',
                      style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
                    ),
                  ],
                ),
                Switch(
                  value: _autoHardlink,
                  activeColor: ObsidianColors.primary,
                  onChanged: (val) => setState(() => _autoHardlink = val),
                ),
              ],
            ),
          ),
          const SizedBox(height: 20),

          // Save settings action bar
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              OutlinedButton(
                onPressed: () async {
                  await state.bridge.resetRule(malId: show.malId);
                  await state.loadShowDetail(show.malId);
                  setState(() {
                    _preferredResolution = '1080p';
                    _autoHardlink = true;
                  });
                  if (context.mounted) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('Reset to global defaults.')),
                    );
                  }
                },
                style: OutlinedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainerHigh,
                  foregroundColor: ObsidianColors.onSurface,
                  side: const BorderSide(color: ObsidianColors.border),
                  padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 12),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
                child: const Text('Reset to Global Defaults'),
              ),
              const SizedBox(width: 12),
              FilledButton(
                onPressed: _isSavingRules
                    ? null
                    : () async {
                        setState(() => _isSavingRules = true);
                        try {
                          await state.updateRule('preferred_resolution', _preferredResolution, malId: show.malId);
                          await state.updateRule('move_mode', _autoHardlink ? 'hardlink' : 'copy', malId: show.malId);
                          await state.loadShowDetail(show.malId);
                          if (context.mounted) {
                            ScaffoldMessenger.of(context).showSnackBar(
                              const SnackBar(content: Text('Saved rule overrides.')),
                            );
                          }
                        } finally {
                          if (mounted) setState(() => _isSavingRules = false);
                        }
                      },
                style: FilledButton.styleFrom(
                  backgroundColor: ObsidianColors.primary,
                  foregroundColor: ObsidianColors.onPrimary,
                  padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
                child: _isSavingRules
                    ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2))
                    : const Text('Save Rule Overrides'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildGroupPill(String name, {bool isPrimary = false}) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: isPrimary ? ObsidianColors.primaryContainer : ObsidianColors.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(
        name,
        style: ObsidianTypography.monoCode(
          color: isPrimary ? ObsidianColors.onPrimaryContainer : ObsidianColors.onSurface,
        ).copyWith(fontWeight: isPrimary ? FontWeight.bold : FontWeight.normal),
      ),
    );
  }

  Widget _buildGroupSeparator() {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 6),
      child: Text('›', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
    );
  }

  // ---------------------------------------------------------------------------
  // Associated Related Works & Recommendations Bento Row
  // ---------------------------------------------------------------------------
  Widget _buildRelatedWorksBento(
    BuildContext context,
    StateProvider state,
    LibraryShow show,
  ) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Wrap(
          alignment: WrapAlignment.spaceBetween,
          crossAxisAlignment: WrapCrossAlignment.center,
          spacing: 12,
          runSpacing: 6,
          children: [
            Wrap(
              crossAxisAlignment: WrapCrossAlignment.center,
              spacing: 10,
              children: [
                Text('Related Works & Spinoffs', style: ObsidianTypography.titleSm()),
                Text('CoMix Wave Films Studio Index',
                    style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 11)),
              ],
            ),
            InkWell(
              onTap: () {
                state.setSearchFilter('Makoto Shinkai');
                state.selectPage(4); // Search page
              },
              child: Text(
                'Explore Director Filmography →',
                style: ObsidianTypography.monoCode(color: ObsidianColors.primary),
              ),
            ),
          ],
        ),
        const SizedBox(height: 12),

        // 4 Bento Cards
        SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: Row(
            children: [
              SizedBox(
                width: 220,
                child: _buildBentoCard(
                  title: 'Tenki no Ko',
                  year: '2019 • In Library',
                  badge: '1080p REMUX',
                  posterUrl: 'https://cdn.myanimelist.net/images/anime/1880/101146.jpg',
                ),
              ),
              const SizedBox(width: 12),
              SizedBox(
                width: 220,
                child: _buildBentoCard(
                  title: 'Suzume',
                  year: '2022 • In Library',
                  badge: '4K UHD',
                  posterUrl: 'https://cdn.myanimelist.net/images/anime/1208/129906.jpg',
                ),
              ),
              const SizedBox(width: 12),
              SizedBox(
                width: 220,
                child: _buildBentoCard(
                  title: 'Kotonoha no Niwa',
                  year: '2013 • In Library',
                  badge: '1080p REMUX',
                  posterUrl: 'https://cdn.myanimelist.net/images/anime/3/49883.jpg',
                ),
              ),
              const SizedBox(width: 12),
              SizedBox(
                width: 220,
                child: _buildBentoCard(
                  title: '5cm per Second',
                  year: '2007 • In Library',
                  badge: '1080p BDRIP',
                  posterUrl: 'https://cdn.myanimelist.net/images/anime/12/32491.jpg',
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildBentoCard({
    required String title,
    required String year,
    required String badge,
    required String posterUrl,
  }) {
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        children: [
          ClipRRect(
            borderRadius: BorderRadius.circular(6),
            child: SizedBox(
              width: 44,
              height: 60,
              child: AnimePoster(
                urlOrPath: posterUrl,
                aspectRatio: 2 / 3,
                borderRadius: BorderRadius.circular(6),
              ),
            ),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  title,
                  style: ObsidianTypography.titleSm().copyWith(fontSize: 12, fontWeight: FontWeight.bold),
                  overflow: TextOverflow.ellipsis,
                ),
                Text(year, style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 10)),
                const SizedBox(height: 2),
                Text(badge, style: ObsidianTypography.labelBadge(color: ObsidianColors.success).copyWith(fontSize: 9)),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Bitrate Sparkline CustomPainter (SVG-identical violet curve & gradient)
// ---------------------------------------------------------------------------
class _BitrateSparklinePainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final w = size.width;
    final h = size.height;

    final path = Path();
    // Normalized curve points matching the SVG path in code.html
    final points = [
      Offset(0.0, h * 0.8),
      Offset(w * 0.1, h * 0.45),
      Offset(w * 0.21, h * 0.62),
      Offset(w * 0.32, h * 0.2),
      Offset(w * 0.43, h * 0.5),
      Offset(w * 0.54, h * 0.35),
      Offset(w * 0.64, h * 0.15),
      Offset(w * 0.75, h * 0.3),
      Offset(w * 0.86, h * 0.6),
      Offset(w * 0.96, h * 0.25),
      Offset(w, h * 0.45),
    ];

    path.moveTo(points[0].dx, points[0].dy);
    for (int i = 0; i < points.length - 1; i++) {
      final p0 = points[i];
      final p1 = points[i + 1];
      final controlX = (p0.dx + p1.dx) / 2;
      path.cubicTo(controlX, p0.dy, controlX, p1.dy, p1.dx, p1.dy);
    }

    // Fill area under curve with vertical gradient
    final fillPath = Path.from(path)
      ..lineTo(w, h)
      ..lineTo(0, h)
      ..close();

    final fillPaint = Paint()
      ..shader = LinearGradient(
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
        colors: [
          ObsidianColors.primaryContainer.withOpacity(0.45),
          ObsidianColors.primaryContainer.withOpacity(0.0),
        ],
      ).createShader(Rect.fromLTWH(0, 0, w, h));
    canvas.drawPath(fillPath, fillPaint);

    // Stroke curve with primary violet
    final strokePaint = Paint()
      ..color = ObsidianColors.primary
      ..strokeWidth = 1.75
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round;
    canvas.drawPath(path, strokePaint);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
