import 'package:flutter/material.dart';

import '../../theme/colors.dart';
import '../../theme/typography.dart';

class WizardHeadlineCard extends StatelessWidget {
  final int plannedEpisodes;
  final int plannedShows;
  final double estimatedStorageGb;
  final String? metadataText;

  const WizardHeadlineCard({
    super.key,
    this.plannedEpisodes = 11,
    this.plannedShows = 11,
    this.estimatedStorageGb = 15.4,
    this.metadataText,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    const Icon(
                      Icons.sync_alt_rounded,
                      size: 16,
                      color: ObsidianColors.primary,
                    ),
                    const SizedBox(width: 6),
                    Text(
                      'QUEUE PIPELINE ANALYSIS',
                      style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.primary,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 6),
                RichText(
                  text: TextSpan(
                    style: ObsidianTypography.headlineLg(),
                    children: [
                      const TextSpan(text: 'This will queue '),
                      TextSpan(
                        text:
                            '$plannedEpisodes episode${plannedEpisodes == 1 ? '' : 's'}',
                        style: const TextStyle(
                          color: ObsidianColors.primary,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      const TextSpan(text: ' across '),
                      TextSpan(
                        text:
                            '$plannedShows show${plannedShows == 1 ? '' : 's'}',
                        style: const TextStyle(
                          color: ObsidianColors.tertiary,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 10),
                Wrap(
                  spacing: 6,
                  runSpacing: 4,
                  children: [
                    _buildPill(
                      '$plannedShows planned',
                      ObsidianColors.surfaceContainerHigh,
                      ObsidianColors.onSurface,
                    ),
                    _buildPill(
                      '$plannedEpisodes Missing Backlog',
                      ObsidianColors.secondary.withOpacity(0.2),
                      ObsidianColors.secondary,
                    ),
                    _buildPill(
                      'Est. Storage ${estimatedStorageGb.toStringAsFixed(estimatedStorageGb.truncateToDouble() == estimatedStorageGb ? 0 : 1)} GB (assumed 1.4 GB/ep)',
                      ObsidianColors.tertiary.withOpacity(0.2),
                      ObsidianColors.tertiary,
                    ),
                    _buildPill(
                      'Nyaa.si (Gate Limiter)',
                      ObsidianColors.surfaceContainerHighest,
                      ObsidianColors.onSurfaceVariant,
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(width: 16),
          // Real Metadata Catalog Telemetry
          Container(
            width: 210,
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: ObsidianColors.border),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Expanded(
                      child: Text(
                        'METADATA CATALOG',
                        style: ObsidianTypography.labelBadge(
                          color: ObsidianColors.primary,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    const SizedBox(width: 4),
                    const Icon(
                      Icons.check_circle_rounded,
                      size: 14,
                      color: ObsidianColors.tertiary,
                    ),
                  ],
                ),
                const SizedBox(height: 6),
                Text(
                  metadataText ?? '21/21 watching, 1040 total',
                  style: ObsidianTypography.monoCode(
                    color: ObsidianColors.onSurface,
                  ).copyWith(fontWeight: FontWeight.bold, fontSize: 12),
                ),
                const SizedBox(height: 4),
                Text(
                  '1,040 anime series synced',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.outline,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildPill(String text, Color bg, Color textCol) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(text, style: ObsidianTypography.labelBadge(color: textCol)),
    );
  }
}
