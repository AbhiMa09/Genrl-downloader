import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../models/queue_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'queue_phase_pill.dart';

class QueueTable extends StatelessWidget {
  final List<QueueItem> items;
  final bool groupSeasons;
  final QueueItem? activeItem;
  final ValueChanged<QueueItem> onSelectItem;

  const QueueTable({
    super.key,
    required this.items,
    this.groupSeasons = true,
    required this.activeItem,
    required this.onSelectItem,
  });

  @override
  Widget build(BuildContext context) {
    if (groupSeasons) {
      final seasons = <(int, int), List<QueueItem>>{};
      for (final item in items) {
        seasons.putIfAbsent((item.malId, item.season), () => []).add(item);
      }
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Queue • ${seasons.length} seasons',
            style: ObsidianTypography.titleSm(),
          ),
          for (final entries in seasons.values)
            Card(
              child: ExpansionTile(
                key: ValueKey(
                  'queue_season_${entries.first.malId}_${entries.first.season}',
                ),
                initiallyExpanded: true,
                title: Text(
                  '${entries.first.showTitle} • Season ${entries.first.season}',
                ),
                subtitle: Text(
                  '${entries.length} queued episode jobs • ${entries.where((e) => e.state == QueueState.completed).length} completed',
                ),
                children: [
                  QueueTable(
                    items: entries,
                    activeItem: activeItem,
                    onSelectItem: onSelectItem,
                    groupSeasons: false,
                  ),
                ],
              ),
            ),
        ],
      );
    }
    return Container(
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      clipBehavior: Clip.antiAlias,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Table Header
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
            color: ObsidianColors.surfaceContainerHighest.withOpacity(0.6),
            child: Row(
              children: [
                Expanded(
                  flex: 5,
                  child: Text(
                    'ANIME & EPISODE',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
                Expanded(
                  flex: 3,
                  child: Text(
                    'STATE / PHASE',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
                Expanded(
                  flex: 2,
                  child: Text(
                    'SEEDERS',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
                Expanded(
                  flex: 3,
                  child: Text(
                    'PROGRESS & SPEED',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
                Expanded(
                  flex: 2,
                  child: Text(
                    'ETA',
                    textAlign: TextAlign.right,
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
                const SizedBox(
                  width: 60,
                  child: Text(
                    'ACTIONS',
                    textAlign: TextAlign.center,
                    style: TextStyle(
                      fontSize: 10,
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
              ],
            ),
          ),

          // Scrollable Queue Rows
          ListView.separated(
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            itemCount: items.length,
            separatorBuilder: (_, __) =>
                const Divider(height: 1, color: ObsidianColors.border),
            itemBuilder: (context, index) {
              final item = items[index];
              final isSelected = activeItem?.id == item.id;

              return InkWell(
                onTap: () => onSelectItem(item),
                child: Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 10,
                  ),
                  decoration: BoxDecoration(
                    color: isSelected
                        ? ObsidianColors.surfaceContainer.withOpacity(0.9)
                        : Colors.transparent,
                    border: isSelected
                        ? const Border(
                            left: BorderSide(
                              color: ObsidianColors.primary,
                              width: 3,
                            ),
                          )
                        : null,
                  ),
                  child: Row(
                    children: [
                      // 1. Anime & Episode (with mini poster thumbnail)
                      Expanded(
                        flex: 5,
                        child: Row(
                          children: [
                            Container(
                              width: 32,
                              height: 48,
                              decoration: BoxDecoration(
                                color: ObsidianColors.surfaceContainerHighest,
                                borderRadius: BorderRadius.circular(4),
                              ),
                              clipBehavior: Clip.antiAlias,
                              child: const Icon(
                                Icons.movie_filter_rounded,
                                size: 16,
                                color: ObsidianColors.outline,
                              ),
                            ),
                            const SizedBox(width: 10),
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                mainAxisAlignment: MainAxisAlignment.center,
                                children: [
                                  Row(
                                    children: [
                                      Expanded(
                                        child: Text(
                                          item.title,
                                          style: ObsidianTypography.titleSm(
                                            color: isSelected
                                                ? ObsidianColors.primary
                                                : ObsidianColors.onSurface,
                                            fontSize: 12,
                                          ),
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis,
                                        ),
                                      ),
                                    ],
                                  ),
                                  const SizedBox(height: 2),
                                  Text(
                                    '${item.episodeLabel} · ${item.releaseName.isNotEmpty ? item.releaseName : item.group}',
                                    style: ObsidianTypography.monoCode(
                                      color: ObsidianColors.outline,
                                      fontSize: 10,
                                    ),
                                    maxLines: 1,
                                    overflow: TextOverflow.ellipsis,
                                  ),
                                  if (item.error != null &&
                                      item.error!.isNotEmpty) ...[
                                    const SizedBox(height: 2),
                                    Text(
                                      item.error!,
                                      style: ObsidianTypography.monoCode(
                                        color: ObsidianColors.error,
                                        fontSize: 10,
                                      ),
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                  ],
                                ],
                              ),
                            ),
                          ],
                        ),
                      ),

                      // 2. State / Phase
                      Expanded(
                        flex: 3,
                        child: Align(
                          alignment: Alignment.centerLeft,
                          child: QueuePhasePill(item: item),
                        ),
                      ),

                      // 3. Seeders
                      Expanded(
                        flex: 2,
                        child: RichText(
                          text: TextSpan(
                            style: ObsidianTypography.monoCode(fontSize: 11),
                            children: [
                              TextSpan(
                                text: '${item.seeders} seeds',
                                style: TextStyle(
                                  color: item.seeders > 10
                                      ? ObsidianColors.tertiary
                                      : ObsidianColors.error,
                                  fontWeight: FontWeight.bold,
                                ),
                              ),
                              TextSpan(
                                text: ' (${item.leechers} leech)',
                                style: const TextStyle(
                                  color: ObsidianColors.outline,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),

                      // 4. Progress & Speed
                      Expanded(
                        flex: 3,
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            ClipRRect(
                              borderRadius: BorderRadius.circular(2),
                              child: LinearProgressIndicator(
                                value: item.progress,
                                minHeight: 4,
                                backgroundColor:
                                    ObsidianColors.surfaceContainerHighest,
                                valueColor: AlwaysStoppedAnimation(
                                  item.state == QueueState.stalled
                                      ? ObsidianColors.error
                                      : (item.state == QueueState.finalizing
                                            ? ObsidianColors.tertiary
                                            : ObsidianColors.primary),
                                ),
                              ),
                            ),
                            const SizedBox(height: 4),
                            Row(
                              mainAxisAlignment: MainAxisAlignment.spaceBetween,
                              children: [
                                Flexible(
                                  child: Text(
                                    item.downloadSpeed,
                                    style: ObsidianTypography.monoCode(
                                      color: ObsidianColors.primary,
                                      fontSize: 10,
                                    ),
                                    overflow: TextOverflow.ellipsis,
                                  ),
                                ),
                                const SizedBox(width: 4),
                                Flexible(
                                  child: Text(
                                    item.formattedSize,
                                    style: ObsidianTypography.monoCode(
                                      color: ObsidianColors.outline,
                                      fontSize: 9,
                                    ),
                                    overflow: TextOverflow.ellipsis,
                                  ),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),

                      // 5. ETA
                      Expanded(
                        flex: 2,
                        child: Text(
                          item.state == QueueState.failed ? 'Failed' : item.eta,
                          textAlign: TextAlign.right,
                          style: ObsidianTypography.monoMetric(
                            color:
                                (item.state == QueueState.stalled ||
                                    item.state == QueueState.failed)
                                ? ObsidianColors.error
                                : ObsidianColors.primary,
                            fontSize: 11,
                          ),
                        ),
                      ),

                      // 6. Actions
                      SizedBox(
                        width: 60,
                        child: Row(
                          mainAxisAlignment: MainAxisAlignment.center,
                          children: [
                            _actionBtn(
                              tooltip: item.state == QueueState.failed
                                  ? 'Retry Job'
                                  : 'Step Job',
                              icon: item.state == QueueState.failed
                                  ? Icons.refresh_rounded
                                  : Icons.restart_alt_rounded,
                              color: item.state == QueueState.failed
                                  ? ObsidianColors.primary
                                  : ObsidianColors.outline,
                              onTap: () async {
                                final p = context.read<StateProvider>();
                                final isFailed =
                                    item.state == QueueState.failed;
                                final ok = isFailed
                                    ? await p.retryJob(item.id)
                                    : await p.recheckJob(item.id);
                                if (context.mounted) {
                                  final verb = isFailed ? 'Retried' : 'Stepped';
                                  ScaffoldMessenger.of(context).showSnackBar(
                                    SnackBar(
                                      content: Text(
                                        ok
                                            ? '$verb job #${item.id}'
                                            : 'Failed to $verb job #${item.id}',
                                      ),
                                    ),
                                  );
                                }
                              },
                            ),
                            _actionBtn(
                              tooltip: 'Cancel Job',
                              icon: Icons.delete_outline_rounded,
                              color: ObsidianColors.outline,
                              onTap: () async {
                                final ok = await context
                                    .read<StateProvider>()
                                    .cancelJob(item.id);
                                if (context.mounted) {
                                  ScaffoldMessenger.of(context).showSnackBar(
                                    SnackBar(
                                      content: Text(
                                        ok
                                            ? 'Cancelled job #${item.id}'
                                            : 'Failed to cancel job #${item.id}',
                                      ),
                                    ),
                                  );
                                }
                              },
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              );
            },
          ),

          // Mini status bar at bottom
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            color: ObsidianColors.surfaceContainerHighest.withOpacity(0.4),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Expanded(
                  child: RichText(
                    overflow: TextOverflow.ellipsis,
                    text: TextSpan(
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline,
                        fontSize: 10,
                      ),
                      children: [
                        const TextSpan(text: 'Selected: '),
                        TextSpan(
                          text: activeItem?.title ?? 'None',
                          style: const TextStyle(
                            color: ObsidianColors.primary,
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                        TextSpan(
                          text:
                              ' | Active Transfers: ${context.watch<StateProvider>().activeDownloads.length}',
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                Text(
                  'Sort: Priority · Descending',
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

  Widget _actionBtn({
    required String tooltip,
    required IconData icon,
    required Color color,
    required VoidCallback onTap,
  }) {
    return Tooltip(
      message: tooltip,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(4.0),
          child: Icon(icon, size: 16, color: color),
        ),
      ),
    );
  }
}
