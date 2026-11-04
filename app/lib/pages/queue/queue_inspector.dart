import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';

import '../../models/queue_item.dart';
import '../../services/state_provider.dart';
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
                      '${item.episodeLabel} • ${item.group}',
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.primary,
                        fontSize: 10,
                      ),
                    ),
                    Text(
                      'Hash: ${item.infoHash != null && item.infoHash!.isNotEmpty ? (item.infoHash!.length > 16 ? "${item.infoHash!.substring(0, 16)}..." : item.infoHash!) : "Pending discovery"}',
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
                onTap: () async {
                  final state = context.read<StateProvider>();
                  final ok = await state.recheckJob(item.id);
                  if (context.mounted) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(
                        content: Text(
                          ok ? 'Stepped job #${item.id}' : 'Failed to step job',
                        ),
                      ),
                    );
                  }
                },
              ),
              _buildSmallActionPill(
                Icons.folder_open_rounded,
                'Open Folder',
                ObsidianColors.outline,
                onTap: () => onOpenFolder(
                  item.releaseName.isNotEmpty ? item.releaseName : item.title,
                ),
              ),
              _buildSmallActionPill(
                Icons.link_rounded,
                'Magnet',
                ObsidianColors.outline,
                onTap: () {
                  if (item.magnet != null && item.magnet!.isNotEmpty) {
                    Clipboard.setData(ClipboardData(text: item.magnet!));
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(
                        content: Text('Magnet link copied to clipboard!'),
                      ),
                    );
                  } else {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(
                        content: Text(
                          'No magnet link available for this job yet',
                        ),
                      ),
                    );
                  }
                },
              ),
              _buildSmallActionPill(
                Icons.cancel_rounded,
                'Abort',
                ObsidianColors.error,
                onTap: () async {
                  final state = context.read<StateProvider>();
                  final ok = await state.cancelJob(item.id);
                  if (context.mounted) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(
                        content: Text(
                          ok
                              ? 'Cancelled job #${item.id}'
                              : 'Failed to cancel job',
                        ),
                      ),
                    );
                  }
                },
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
                'Job #${item.id}',
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
                _buildMetricRow('Uploading', item.uploadSpeed),
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
                    item.releaseName.isNotEmpty
                        ? item.releaseName
                        : (item.title.isNotEmpty ? '${item.title}.mkv' : '—'),
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
                'Policy Matcher',
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.outline,
                  fontSize: 10,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),

          FutureBuilder<List<CandidateItem>>(
            future: context.read<StateProvider>().bridge.getCandidates(
              item.malId,
            ),
            builder: (context, snapshot) {
              final candidates = snapshot.data ?? [];
              if (candidates.isEmpty) {
                return Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(
                    item.releaseName.isNotEmpty
                        ? 'Candidate: ${item.releaseName}\nGroup: ${item.group} • ${item.seeders} seeds'
                        : 'Searching indexers for candidates...',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.outline,
                      fontSize: 10,
                    ),
                  ),
                );
              }

              return Column(
                children: candidates.take(3).map((c) {
                  final isChosen =
                      !c.isRejected &&
                      (item.releaseName.contains(c.name) ||
                          c == candidates.first);
                  return Container(
                    margin: const EdgeInsets.only(bottom: 6),
                    padding: const EdgeInsets.all(10),
                    decoration: BoxDecoration(
                      color: isChosen
                          ? ObsidianColors.primaryContainer.withOpacity(0.15)
                          : ObsidianColors.surfaceContainer,
                      borderRadius: BorderRadius.circular(8),
                      border: isChosen
                          ? Border.all(
                              color: ObsidianColors.primary.withOpacity(0.3),
                            )
                          : null,
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
                                      color: isChosen
                                          ? ObsidianColors.primary
                                          : ObsidianColors
                                                .surfaceContainerHighest,
                                      borderRadius: BorderRadius.circular(3),
                                    ),
                                    child: Text(
                                      isChosen
                                          ? 'CHOSEN'
                                          : (c.isRejected
                                                ? 'REJECTED'
                                                : 'TIED'),
                                      style: ObsidianTypography.labelBadge(
                                        color: isChosen
                                            ? ObsidianColors.onPrimary
                                            : ObsidianColors.outline,
                                        fontSize: 9,
                                      ),
                                    ),
                                  ),
                                  const SizedBox(width: 6),
                                  Flexible(
                                    child: Text(
                                      c.name,
                                      style: ObsidianTypography.titleSm(
                                        fontSize: 11,
                                      ),
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                  ),
                                ],
                              ),
                            ),
                            const SizedBox(width: 6),
                            Text(
                              '${c.score.toInt()} / 100',
                              style: ObsidianTypography.monoMetric(
                                color: isChosen
                                    ? ObsidianColors.primary
                                    : ObsidianColors.outline,
                                fontWeight: FontWeight.bold,
                                fontSize: 11,
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 4),
                        Row(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            Expanded(
                              child: Text(
                                'Group: ${c.group ?? "-"} • ${c.resolution ?? "-"} • ${c.formattedSize}',
                                style: ObsidianTypography.monoCode(
                                  color: ObsidianColors.outline,
                                  fontSize: 9,
                                ),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                            const SizedBox(width: 6),
                            Text(
                              '${c.seeders} seeds (${c.leechers} leech)',
                              style: ObsidianTypography.monoMetric(
                                color: ObsidianColors.tertiary,
                                fontSize: 9,
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                  );
                }).toList(),
              );
            },
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
