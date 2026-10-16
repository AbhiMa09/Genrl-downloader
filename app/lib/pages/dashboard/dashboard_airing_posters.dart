import 'dart:io';

import 'package:flutter/material.dart';

import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class DashboardAiringPosters extends StatelessWidget {
  final StateProvider state;

  const DashboardAiringPosters({super.key, required this.state});

  void _openFileExplorer(String path) {
    if (!Platform.environment.containsKey('FLUTTER_TEST')) {
      Process.run('xdg-open', [path]);
    }
  }

  @override
  Widget build(BuildContext context) {
    if (state.shows.isEmpty) {
      return Container(
        padding: const EdgeInsets.all(20),
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainerLow,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: ObsidianColors.border),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                const Icon(
                  Icons.live_tv_rounded,
                  size: 20,
                  color: ObsidianColors.primary,
                ),
                const SizedBox(width: 8),
                Text(
                  'Airing Today & Monitored Next Up',
                  style: ObsidianTypography.headlineMd(),
                ),
              ],
            ),
            const SizedBox(height: 20),
            Center(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Text(
                  'No organized shows discovered yet. Run a disk scan or connect MAL to populate.',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurfaceVariant,
                  ),
                  textAlign: TextAlign.center,
                ),
              ),
            ),
          ],
        ),
      );
    }

    final shows = state.shows.take(5).toList();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Wrap(
          alignment: WrapAlignment.spaceBetween,
          crossAxisAlignment: WrapCrossAlignment.center,
          spacing: 12,
          runSpacing: 8,
          children: [
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(
                  Icons.live_tv_rounded,
                  size: 20,
                  color: ObsidianColors.primary,
                ),
                const SizedBox(width: 8),
                Text(
                  'Airing Today & Monitored Next Up',
                  style: ObsidianTypography.headlineMd(),
                ),
              ],
            ),
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  'Auto-snatch: ',
                  style: ObsidianTypography.monoCode(
                    color: ObsidianColors.onSurfaceVariant,
                    fontSize: 11,
                  ),
                ),
                Text(
                  'ENABLED',
                  style: ObsidianTypography.monoCode(
                    color: ObsidianColors.tertiary,
                    fontSize: 11,
                    fontWeight: FontWeight.bold,
                  ),
                ),
                const SizedBox(width: 14),
                InkWell(
                  onTap: () => state.selectPage(3), // Library
                  child: Row(
                    children: [
                      Text(
                        'Full Catalog',
                        style: ObsidianTypography.bodySm(
                          color: ObsidianColors.primary,
                        ),
                      ),
                      const Icon(
                        Icons.arrow_forward_rounded,
                        size: 14,
                        color: ObsidianColors.primary,
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ],
        ),
        const SizedBox(height: 12),

        // 2:3 Media Poster Grid
        LayoutBuilder(
          builder: (context, constraints) {
            final columns = constraints.maxWidth > 1100
                ? 5
                : constraints.maxWidth > 750
                ? 3
                : 2;
            final itemWidth =
                (constraints.maxWidth - (columns - 1) * 12) / columns;

            return Wrap(
              spacing: 12,
              runSpacing: 12,
              children: shows.map((show) {
                return SizedBox(
                  width: itemWidth,
                  child: DashboardPosterCard(
                    show: show,
                    onTap: () => state.openShowDetail(show),
                    onOpenFolder: () => _openFileExplorer(
                      show.folderPath ?? '/mnt/anime/TV/${show.title}',
                    ),
                    onRefreshMeta: () => state.forceSyncMal(),
                    onSearch: () {
                      state.selectPage(4);
                    },
                  ),
                );
              }).toList(),
            );
          },
        ),
      ],
    );
  }
}

class DashboardPosterCard extends StatefulWidget {
  final LibraryShow show;
  final VoidCallback onTap;
  final VoidCallback onOpenFolder;
  final VoidCallback onRefreshMeta;
  final VoidCallback onSearch;

  const DashboardPosterCard({
    super.key,
    required this.show,
    required this.onTap,
    required this.onOpenFolder,
    required this.onRefreshMeta,
    required this.onSearch,
  });

  @override
  State<DashboardPosterCard> createState() => _DashboardPosterCardState();
}

class _DashboardPosterCardState extends State<DashboardPosterCard> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final show = widget.show;

    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: InkWell(
        onTap: widget.onTap,
        borderRadius: BorderRadius.circular(12),
        child: Container(
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainerLow,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(
              color: _isHovered
                  ? ObsidianColors.primary.withOpacity(0.6)
                  : ObsidianColors.border,
            ),
            boxShadow: _isHovered
                ? [
                    BoxShadow(
                      color: ObsidianColors.primary.withOpacity(0.2),
                      blurRadius: 16,
                      offset: const Offset(0, 4),
                    ),
                  ]
                : null,
          ),
          clipBehavior: Clip.antiAlias,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // 2:3 Aspect Ratio Poster Container
              AspectRatio(
                aspectRatio: 2 / 3,
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    if (show.posterUrl != null && show.posterUrl!.isNotEmpty)
                      Image.network(
                        show.posterUrl!,
                        fit: BoxFit.cover,
                        errorBuilder: (_, __, ___) =>
                            _buildPosterPlaceholder(show),
                      )
                    else
                      _buildPosterPlaceholder(show),

                    // Scrim gradient
                    Container(
                      decoration: const BoxDecoration(
                        gradient: LinearGradient(
                          begin: Alignment.topCenter,
                          end: Alignment.bottomCenter,
                          stops: [0.0, 0.5, 1.0],
                          colors: [
                            Colors.black38,
                            Colors.transparent,
                            Colors.black87,
                          ],
                        ),
                      ),
                    ),

                    // Top Badges
                    Positioned(
                      top: 6,
                      left: 6,
                      child: Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 5,
                          vertical: 2,
                        ),
                        decoration: BoxDecoration(
                          color: ObsidianColors.surfaceContainerLowest
                              .withOpacity(0.85),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          show.episodesDownloaded == show.episodesTotal
                              ? 'Complete'
                              : '1080p Remux',
                          style: ObsidianTypography.labelBadge(
                            color: show.episodesDownloaded == show.episodesTotal
                                ? ObsidianColors.success
                                : ObsidianColors.tertiary,
                            fontSize: 9,
                          ),
                        ),
                      ),
                    ),

                    Positioned(
                      top: 6,
                      right: 6,
                      child: Container(
                        width: 22,
                        height: 22,
                        decoration: BoxDecoration(
                          color: ObsidianColors.surfaceContainerLowest
                              .withOpacity(0.85),
                          shape: BoxShape.circle,
                        ),
                        child: const Icon(
                          Icons.bookmark_rounded,
                          size: 14,
                          color: ObsidianColors.primary,
                        ),
                      ),
                    ),

                    // Hover Quick Action Micro-Dock
                    if (_isHovered)
                      Positioned(
                        top: 36,
                        left: 10,
                        right: 10,
                        child: Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 4,
                            vertical: 2,
                          ),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainerLowest
                                .withOpacity(0.92),
                            borderRadius: BorderRadius.circular(6),
                            border: Border.all(color: ObsidianColors.border),
                          ),
                          child: Row(
                            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                            children: [
                              InkWell(
                                onTap: widget.onSearch,
                                child: const Icon(
                                  Icons.search_rounded,
                                  size: 16,
                                  color: ObsidianColors.onSurface,
                                ),
                              ),
                              InkWell(
                                onTap: widget.onRefreshMeta,
                                child: const Icon(
                                  Icons.refresh_rounded,
                                  size: 16,
                                  color: ObsidianColors.onSurface,
                                ),
                              ),
                              InkWell(
                                onTap: widget.onOpenFolder,
                                child: const Icon(
                                  Icons.folder_open_rounded,
                                  size: 16,
                                  color: ObsidianColors.onSurface,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),

                    // Bottom Poster Meta
                    Positioned(
                      bottom: 8,
                      left: 8,
                      right: 8,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            mainAxisAlignment: MainAxisAlignment.spaceBetween,
                            children: [
                              Text(
                                'EP ${show.downloadedEpisodes} / ${show.totalEpisodes}',
                                style: ObsidianTypography.monoMetric(
                                  color: ObsidianColors.tertiary,
                                  fontSize: 10,
                                ),
                              ),
                              Text(
                                '★ ${show.score.toStringAsFixed(1)}',
                                style: ObsidianTypography.monoMetric(
                                  color: ObsidianColors.primary,
                                  fontSize: 10,
                                  fontWeight: FontWeight.bold,
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 2),
                          Text(
                            show.title,
                            style: ObsidianTypography.titleSm(
                              color: Colors.white,
                              fontSize: 12,
                            ),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          Text(
                            show.status == 'Airing'
                                ? 'Airs weekly'
                                : 'Complete',
                            style: ObsidianTypography.monoCode(
                              color: ObsidianColors.onSurfaceVariant,
                              fontSize: 9,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildPosterPlaceholder(LibraryShow show) {
    return Container(
      color: ObsidianColors.surfaceContainerHighest,
      child: Center(
        child: Text(
          show.title.isNotEmpty ? show.title[0] : '?',
          style: ObsidianTypography.headlineLg(color: ObsidianColors.outline),
        ),
      ),
    );
  }
}
