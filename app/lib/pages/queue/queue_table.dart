import 'package:flutter/material.dart';

import '../../models/queue_item.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../widgets/loading/inline_spinner.dart';

class QueueTable extends StatelessWidget {
  final List<QueueItem> items;
  final QueueItem? activeItem;
  final ValueChanged<QueueItem> onSelectItem;

  const QueueTable({
    super.key,
    required this.items,
    required this.activeItem,
    required this.onSelectItem,
  });

  @override
  Widget build(BuildContext context) {
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
                          child: _buildPhasePill(item),
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
                          item.eta,
                          textAlign: TextAlign.right,
                          style: ObsidianTypography.monoMetric(
                            color: item.state == QueueState.stalled
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
                            InkWell(
                              onTap: () {},
                              child: const Padding(
                                padding: EdgeInsets.all(4.0),
                                child: Icon(
                                  Icons.pause_rounded,
                                  size: 16,
                                  color: ObsidianColors.outline,
                                ),
                              ),
                            ),
                            InkWell(
                              onTap: () {},
                              child: const Padding(
                                padding: EdgeInsets.all(4.0),
                                child: Icon(
                                  Icons.delete_outline_rounded,
                                  size: 16,
                                  color: ObsidianColors.outline,
                                ),
                              ),
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
                        const TextSpan(
                          text: ' | qBittorrent Slot Allocation: 4/12',
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

  Widget _buildPhasePill(QueueItem item) {
    Color bg = ObsidianColors.primaryContainer.withOpacity(0.2);
    Color fg = ObsidianColors.primary;
    String label = 'Downloading ${(item.progress * 100).toInt()}%';

    if (item.state == QueueState.finalizing) {
      bg = ObsidianColors.tertiaryContainer.withOpacity(0.2);
      fg = ObsidianColors.tertiary;
      label = 'Hardlinking Media';
    } else if (item.state == QueueState.queued) {
      bg = ObsidianColors.surfaceContainerHighest;
      fg = ObsidianColors.outline;
      label = 'Searching Indexers';
    } else if (item.state == QueueState.blocked) {
      bg = ObsidianColors.error.withOpacity(0.2);
      fg = ObsidianColors.error;
      label = 'Blocked: Low Inodes';
    } else if (item.state == QueueState.stalled) {
      bg = ObsidianColors.error.withOpacity(0.3);
      fg = ObsidianColors.error;
      label = 'Tracker Timeout (504)';
    } else if (item.seeders <= 1) {
      bg = ObsidianColors.secondaryContainer.withOpacity(0.4);
      fg = ObsidianColors.secondary;
      label = 'Pick Alternative (1 seed)';
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (item.state == QueueState.queued) ...[
            const InlineSpinner(size: 9, color: ObsidianColors.outline),
            const SizedBox(width: 4),
          ],
          Flexible(
            child: Text(
              label,
              style: ObsidianTypography.monoCode(
                color: fg,
                fontSize: 10,
                fontWeight: FontWeight.w500,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}
