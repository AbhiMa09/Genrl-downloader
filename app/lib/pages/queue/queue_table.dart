import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../models/queue_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'queue_table_row.dart';

/// "3 episode jobs • 1 completed • 1 failed" — counts by real state.
String _seasonSummary(List<QueueItem> entries) {
  int count(QueueState s) => entries.where((e) => e.state == s).length;
  final jobs = entries.length == 1
      ? '1 episode job'
      : '${entries.length} episode jobs';
  final failed = count(QueueState.failed);
  return '$jobs • ${count(QueueState.completed)} completed'
      '${failed > 0 ? ' • $failed failed' : ''}';
}

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
                subtitle: Text(_seasonSummary(entries)),
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
                    'MEDIA & RELEASE',
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
              return QueueTableRow(
                item: item,
                isSelected: activeItem?.id == item.id,
                onTap: () => onSelectItem(item),
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
}
