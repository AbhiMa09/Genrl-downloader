import 'package:flutter/material.dart';

import '../../models/library_item.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../widgets/loading/skeleton_poster_card.dart';
import 'poster_density.dart';

class LibraryPosterCard extends StatefulWidget {
  final LibraryShow show;
  final PosterDensity density;
  final VoidCallback onTap;
  final VoidCallback onOpenFolder;
  final VoidCallback onToggleAuto;

  const LibraryPosterCard({
    super.key,
    required this.show,
    required this.density,
    required this.onTap,
    required this.onOpenFolder,
    required this.onToggleAuto,
  });

  @override
  State<LibraryPosterCard> createState() => _LibraryPosterCardState();
}

class _LibraryPosterCardState extends State<LibraryPosterCard> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final show = widget.show;
    final downloaded = show.episodesDownloaded;
    final total = show.episodesTotal;
    final percent = (downloaded / (total > 0 ? total : 1)).clamp(0.0, 1.0);
    final percentInt = (percent * 100).toInt();

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
              // 2:3 Poster
              AspectRatio(
                aspectRatio: 2 / 3,
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    if (show.posterUrl != null && show.posterUrl!.isNotEmpty)
                      Image.network(
                        show.posterUrl!,
                        fit: BoxFit.cover,
                        loadingBuilder: (context, child, progress) {
                          if (progress == null) return child;
                          return const SkeletonPosterCard();
                        },
                        errorBuilder: (_, __, ___) => _buildPlaceholder(show),
                      )
                    else
                      _buildPlaceholder(show),

                    // Scrim
                    Container(
                      decoration: const BoxDecoration(
                        gradient: LinearGradient(
                          begin: Alignment.topCenter,
                          end: Alignment.bottomCenter,
                          stops: [0.0, 0.4, 1.0],
                          colors: [
                            Colors.black38,
                            Colors.transparent,
                            Colors.black87,
                          ],
                        ),
                      ),
                    ),

                    // Top Left Badges
                    Positioned(
                      top: 6,
                      left: 6,
                      child: Row(
                        children: [
                          Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 5,
                              vertical: 2,
                            ),
                            decoration: BoxDecoration(
                              color: ObsidianColors.secondaryContainer
                                  .withOpacity(0.9),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text(
                              'AUTO',
                              style: ObsidianTypography.labelBadge(
                                color: ObsidianColors.onSecondaryContainer,
                                fontSize: 8,
                              ),
                            ),
                          ),
                          const SizedBox(width: 4),
                          Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 5,
                              vertical: 2,
                            ),
                            decoration: BoxDecoration(
                              color: show.status == 'Completed'
                                  ? ObsidianColors.surfaceContainerHighest
                                        .withOpacity(0.85)
                                  : ObsidianColors.tertiaryContainer
                                        .withOpacity(0.85),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text(
                              show.status == 'Completed'
                                  ? 'COMPLETED'
                                  : 'AIRING',
                              style: ObsidianTypography.labelBadge(
                                color: show.status == 'Completed'
                                    ? ObsidianColors.onSurfaceVariant
                                    : ObsidianColors.onTertiary,
                                fontSize: 8,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),

                    // Top Right Quick Hover Action
                    if (_isHovered)
                      Positioned(
                        top: 6,
                        right: 6,
                        child: Container(
                          padding: const EdgeInsets.all(2),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainerHighest
                                .withOpacity(0.9),
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Row(
                            children: [
                              InkWell(
                                onTap: widget.onOpenFolder,
                                child: const Padding(
                                  padding: EdgeInsets.all(3.0),
                                  child: Icon(
                                    Icons.folder_open_rounded,
                                    size: 14,
                                    color: ObsidianColors.onSurfaceVariant,
                                  ),
                                ),
                              ),
                              InkWell(
                                onTap: widget.onToggleAuto,
                                child: const Padding(
                                  padding: EdgeInsets.all(3.0),
                                  child: Icon(
                                    Icons.bolt_rounded,
                                    size: 14,
                                    color: ObsidianColors.onSurfaceVariant,
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),

                    // Bottom Quality Badge
                    Positioned(
                      bottom: 6,
                      left: 6,
                      right: 6,
                      child: Text(
                        '1080p HEVC · Erai',
                        style: ObsidianTypography.monoCode(
                          color: ObsidianColors.tertiary,
                          fontSize: 9,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                  ],
                ),
              ),

              // Card Bottom Metadata
              Padding(
                padding: const EdgeInsets.all(10),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      show.title,
                      style: ObsidianTypography.titleSm(
                        fontSize: widget.density == PosterDensity.compact
                            ? 11
                            : 13,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 2),
                    Text(
                      'Season ${show.season} · ${show.studio ?? "Anime"}',
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline,
                        fontSize: 10,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 8),

                    // Progress Bar & Counter
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Text(
                          '$downloaded / $total eps',
                          style: ObsidianTypography.monoCode(
                            color: ObsidianColors.onSurfaceVariant,
                            fontSize: 10,
                          ),
                        ),
                        Text(
                          '$percentInt%',
                          style: ObsidianTypography.monoMetric(
                            color: ObsidianColors.tertiary,
                            fontWeight: FontWeight.bold,
                            fontSize: 10,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 4),
                    ClipRRect(
                      borderRadius: BorderRadius.circular(2),
                      child: LinearProgressIndicator(
                        value: percent,
                        minHeight: 4,
                        backgroundColor: ObsidianColors.surfaceContainerHighest,
                        valueColor: const AlwaysStoppedAnimation(
                          ObsidianColors.primary,
                        ),
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

  Widget _buildPlaceholder(LibraryShow show) {
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
