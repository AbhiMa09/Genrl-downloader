import 'package:flutter/material.dart';

import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../widgets/anime_poster.dart';
import 'bitrate_sparkline_card.dart';
import 'dialogs.dart';
import 'storage_inspector_card.dart';
import 'watchlist_controller_card.dart';

class ShowDetailLeftRail extends StatelessWidget {
  final StateProvider state;
  final LibraryShow show;
  final bool isMovie;
  final String formatLabel;
  final String durationLabel;
  final String premieredLabel;
  final bool isFinished;
  final String onDiskString;

  const ShowDetailLeftRail({
    super.key,
    required this.state,
    required this.show,
    required this.isMovie,
    required this.formatLabel,
    required this.durationLabel,
    required this.premieredLabel,
    required this.isFinished,
    required this.onDiskString,
  });

  @override
  Widget build(BuildContext context) {
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
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 3,
                      ),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerLowest
                            .withOpacity(0.9),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        'BD 1080p',
                        style: ObsidianTypography.labelBadge(
                          color: ObsidianColors.tertiary,
                        ),
                      ),
                    ),
                    const SizedBox(width: 6),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 3,
                      ),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerLowest
                            .withOpacity(0.9),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        'HEVC',
                        style: ObsidianTypography.labelBadge(
                          color: ObsidianColors.primary,
                        ),
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
                      color: ObsidianColors.surfaceContainerLowest.withOpacity(
                        0.9,
                      ),
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
                  padding: const EdgeInsets.symmetric(
                    horizontal: 14,
                    vertical: 10,
                  ),
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
                        padding: const EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 2,
                        ),
                        decoration: BoxDecoration(
                          color: ObsidianColors.secondaryContainer.withOpacity(
                            0.5,
                          ),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          isFinished
                              ? (isMovie ? 'MOVIE COMPLETE' : 'SEASON COMPLETE')
                              : 'AIRING',
                          style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.secondary,
                          ),
                        ),
                      ),
                      Text(
                        durationLabel,
                        style: ObsidianTypography.monoCode(
                          color: ObsidianColors.onSurfaceVariant,
                        ),
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
                    onTap: () => showRatingDialog(context, state, show),
                    borderRadius: BorderRadius.circular(8),
                    child: Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 10,
                        vertical: 4,
                      ),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerLowest,
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(
                          color: ObsidianColors.warning.withOpacity(0.35),
                        ),
                      ),
                      child: Row(
                        children: [
                          const Icon(
                            Icons.star_rounded,
                            size: 18,
                            color: ObsidianColors.warning,
                          ),
                          const SizedBox(width: 4),
                          Text(
                            show.score.toStringAsFixed(1),
                            style: ObsidianTypography.monoMetric(
                              color: ObsidianColors.onSurface,
                            ).copyWith(fontWeight: FontWeight.bold),
                          ),
                          const SizedBox(width: 6),
                          Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 4,
                              vertical: 1,
                            ),
                            decoration: BoxDecoration(
                              color: ObsidianColors.primary.withOpacity(0.15),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text(
                              'Edit',
                              style: ObsidianTypography.monoCode(
                                color: ObsidianColors.primary,
                                fontSize: 9,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                  Text.rich(
                    TextSpan(
                      text: 'Ranked ',
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline,
                      ),
                      children: [
                        TextSpan(
                          text: '#${show.rank ?? 12}',
                          style: ObsidianTypography.monoCode(
                            color: ObsidianColors.onSurface,
                          ).copyWith(fontWeight: FontWeight.bold),
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
                  Expanded(child: _buildTelemetryCell('FORMAT', formatLabel)),
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
                      statusColor: isFinished
                          ? ObsidianColors.success
                          : ObsidianColors.tertiary,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: _buildTelemetryCell('STUDIO', show.studio ?? '-'),
                  ),
                ],
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),

        // User Watchlist & Progress Controller Card
        WatchlistControllerCard(state: state, show: show),
        const SizedBox(height: 16),

        // File Storage Inspector Card
        StorageInspectorCard(
          state: state,
          show: show,
          onDiskString: onDiskString,
        ),
        const SizedBox(height: 16),

        // Quick Telemetry Graph: Codec Bitrate Analysis Sparkline
        BitrateSparklineCard(durationLabel: durationLabel),
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
            style: ObsidianTypography.monoCode(color: ObsidianColors.outline)
                .copyWith(fontSize: 10),
          ),
          const SizedBox(height: 2),
          if (statusColor != null)
            Row(
              children: [
                Container(
                  width: 6,
                  height: 6,
                  decoration: BoxDecoration(
                    color: statusColor,
                    shape: BoxShape.circle,
                  ),
                ),
                const SizedBox(width: 4),
                Expanded(
                  child: Text(
                    value,
                    style: ObsidianTypography.titleSm(color: statusColor)
                        .copyWith(fontSize: 12),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            )
          else
            Text(
              value,
              style: ObsidianTypography.titleSm(color: ObsidianColors.onSurface)
                  .copyWith(fontSize: 12),
              overflow: TextOverflow.ellipsis,
            ),
        ],
      ),
    );
  }
}
