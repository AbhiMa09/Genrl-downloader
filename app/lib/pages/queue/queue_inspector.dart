import 'package:flutter/material.dart';

import '../../models/queue_item.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class QueueInspector extends StatelessWidget {
  final QueueItem item;
  final void Function(String path) onOpenFolder;

  const QueueInspector({
    super.key,
    required this.item,
    required this.onOpenFolder,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Header
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Container(
                width: 48,
                height: 64,
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(6),
                ),
                clipBehavior: Clip.antiAlias,
                child: const Icon(
                  Icons.movie_rounded,
                  size: 24,
                  color: ObsidianColors.outline,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      item.title,
                      style: ObsidianTypography.titleSm(fontSize: 13),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 2),
                    Text(
                      'Finale Arc • High Priority',
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.primary,
                        fontSize: 10,
                      ),
                    ),
                    Text(
                      'Hash: 4a8f9c2e0b11...d81e05a',
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline,
                        fontSize: 9,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),

          // Quick Action Pill Strip
          Wrap(
            spacing: 6,
            runSpacing: 6,
            children: [
              _buildSmallActionPill(
                Icons.replay_rounded,
                'Force Recheck',
                ObsidianColors.tertiary,
              ),
              _buildSmallActionPill(
                Icons.folder_open_rounded,
                'Open Folder',
                ObsidianColors.outline,
                onTap: () => onOpenFolder('/mnt/anime/TV'),
              ),
              _buildSmallActionPill(
                Icons.link_rounded,
                'Magnet',
                ObsidianColors.outline,
              ),
              _buildSmallActionPill(
                Icons.cancel_rounded,
                'Abort',
                ObsidianColors.error,
              ),
            ],
          ),
          const SizedBox(height: 16),
          const Divider(height: 1, color: ObsidianColors.border),
          const SizedBox(height: 16),

          // Section 1: Swarm & Target Ingestion
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text(
                  'SWARM & TARGET INGESTION',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.outline,
                    fontSize: 10,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 8),
              Text(
                'qBit Slot #2',
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.primary,
                  fontSize: 10,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Container(
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainer,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _buildMetricRow(
                  'Total Size',
                  '${item.formattedSize} (${(item.progress * 100).toInt()}% done)',
                ),
                _buildMetricRow(
                  'Peers Swarm',
                  '${item.seeders} seeds · ${item.leechers} leech',
                ),
                _buildMetricRow(
                  'Share Ratio',
                  '0.14 (Uploading ${item.uploadSpeed})',
                ),
                _buildMetricRow(
                  'ETA Countdown',
                  '~${item.eta} remaining',
                  valueColor: ObsidianColors.primary,
                ),
                const SizedBox(height: 6),
                Text(
                  'Destination Hardlink Target',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.outline,
                    fontSize: 9,
                  ),
                ),
                const SizedBox(height: 3),
                Container(
                  padding: const EdgeInsets.all(6),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLowest,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    '/mnt/anime/TV/${item.title}.mkv',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.onSurfaceVariant,
                      fontSize: 9,
                    ),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),

          // Section 2: Automated Decision Matrix
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text(
                  'AUTOMATED DECISION MATRIX',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.outline,
                    fontSize: 10,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 8),
              Text(
                'Prowlarr Parser v3',
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.tertiary,
                  fontSize: 10,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),

          // Candidate 1 (Chosen)
          Container(
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.primaryContainer.withOpacity(0.15),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(
                color: ObsidianColors.primary.withOpacity(0.3),
              ),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Expanded(
                      child: Row(
                        children: [
                          Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 4,
                              vertical: 1.5,
                            ),
                            decoration: BoxDecoration(
                              color: ObsidianColors.primary,
                              borderRadius: BorderRadius.circular(3),
                            ),
                            child: Text(
                              'CHOSEN',
                              style: ObsidianTypography.labelBadge(
                                color: ObsidianColors.onPrimary,
                                fontSize: 9,
                              ),
                            ),
                          ),
                          const SizedBox(width: 6),
                          Flexible(
                            child: Text(
                              '[SubsPlease] 1080p WEBRip Opus',
                              style: ObsidianTypography.titleSm(fontSize: 11),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 6),
                    Text(
                      '98 / 100',
                      style: ObsidianTypography.monoMetric(
                        color: ObsidianColors.primary,
                        fontWeight: FontWeight.bold,
                        fontSize: 12,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  'Tier-1 group • Native 1080p rip • Opus Japanese 2.0 • Fast seed availability.',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurfaceVariant,
                    fontSize: 10,
                  ),
                ),
                const SizedBox(height: 4),
                Wrap(
                  spacing: 8,
                  children: [
                    Text(
                      '+40 Group Tier',
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline,
                        fontSize: 9,
                      ),
                    ),
                    Text(
                      '+30 Audio Match',
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline,
                        fontSize: 9,
                      ),
                    ),
                    Text(
                      '+28 Seeder Density',
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline,
                        fontSize: 9,
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 6),

          // Candidate 2 (Rejected)
          Container(
            padding: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainer,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 4,
                              vertical: 1,
                            ),
                            decoration: BoxDecoration(
                              color: ObsidianColors.surfaceContainerHighest,
                              borderRadius: BorderRadius.circular(3),
                            ),
                            child: Text(
                              'REJECTED',
                              style: ObsidianTypography.labelBadge(
                                color: ObsidianColors.outline,
                                fontSize: 8,
                              ),
                            ),
                          ),
                          const SizedBox(width: 6),
                          Flexible(
                            child: Text(
                              '[Erai-raws] 1080p HEVC',
                              style: ObsidianTypography.bodySm(fontSize: 10),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ],
                      ),
                      Text(
                        'Equal quality, but initial swarm count 48 vs 142.',
                        style: ObsidianTypography.bodySm(
                          color: ObsidianColors.outline,
                          fontSize: 9,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 6),
                Text(
                  '92 / 100',
                  style: ObsidianTypography.monoCode(
                    color: ObsidianColors.outline,
                    fontSize: 10,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildMetricRow(String label, String value, {Color? valueColor}) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Expanded(
            child: Text(
              label,
              style: ObsidianTypography.monoCode(
                color: ObsidianColors.outline,
                fontSize: 10,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
          const SizedBox(width: 8),
          Flexible(
            child: Text(
              value,
              style: ObsidianTypography.monoMetric(
                color: valueColor ?? ObsidianColors.onSurface,
                fontSize: 10,
              ),
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.right,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildSmallActionPill(
    IconData icon,
    String label,
    Color iconColor, {
    VoidCallback? onTap,
  }) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(6),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainerHigh,
          borderRadius: BorderRadius.circular(6),
          border: Border.all(color: ObsidianColors.border, width: 0.5),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 12, color: iconColor),
            const SizedBox(width: 4),
            Text(
              label,
              style: ObsidianTypography.monoCode(
                color: ObsidianColors.onSurface,
                fontSize: 10,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
