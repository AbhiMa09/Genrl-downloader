import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class DashboardPipelineTable extends StatelessWidget {
  final StateProvider state;

  const DashboardPipelineTable({super.key, required this.state});

  String? _extractGroup(String title) {
    if (title.startsWith('[') && title.contains(']')) {
      final end = title.indexOf(']');
      return title.substring(1, end);
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    if (state.queue.isEmpty) {
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
                  Icons.downloading_rounded,
                  size: 20,
                  color: ObsidianColors.primary,
                ),
                const SizedBox(width: 8),
                Text(
                  'Daemon Pipeline & Active Ingestion',
                  style: ObsidianTypography.headlineMd(),
                ),
                const SizedBox(width: 10),
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 2,
                  ),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    'ACTIVE DOWNLOAD TRANSFERS',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.primary,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 20),
            Center(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Text(
                  'Queue is empty. Automation is watching for newly aired releases.',
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

    final queueItems = state.queue;

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
          Wrap(
            alignment: WrapAlignment.spaceBetween,
            crossAxisAlignment: WrapCrossAlignment.center,
            spacing: 12,
            runSpacing: 8,
            children: [
              Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 8,
                runSpacing: 4,
                children: [
                  const Icon(
                    Icons.downloading_rounded,
                    size: 20,
                    color: ObsidianColors.primary,
                  ),
                  Text(
                    'Daemon Pipeline & Active Ingestion',
                    style: ObsidianTypography.headlineMd(),
                  ),
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 2,
                    ),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerHighest,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      'ACTIVE DOWNLOAD TRANSFERS',
                      style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.primary,
                      ),
                    ),
                  ),
                ],
              ),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  OutlinedButton.icon(
                    onPressed: () => state.toggleAutomation(),
                    icon: Icon(
                      state.status.automationOn
                          ? Icons.pause_rounded
                          : Icons.play_arrow_rounded,
                      size: 14,
                    ),
                    label: Text(
                      state.status.automationOn ? 'Pause All' : 'Resume',
                      style: ObsidianTypography.monoCode(fontSize: 11),
                    ),
                    style: OutlinedButton.styleFrom(
                      side: const BorderSide(color: ObsidianColors.border),
                      padding: const EdgeInsets.symmetric(
                        horizontal: 10,
                        vertical: 6,
                      ),
                    ),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 14),

          // Dense Table View
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              horizontalMargin: 8,
              columnSpacing: 16,
              headingRowHeight: 32,
              dataRowMinHeight: 48,
              dataRowMaxHeight: 56,
              headingRowColor: WidgetStateProperty.all(
                ObsidianColors.surfaceContainerLowest,
              ),
              columns: [
                DataColumn(
                  label: Text(
                    'PAYLOAD / SERIES',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
                DataColumn(
                  label: Text(
                    'PROGRESS',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
                DataColumn(
                  label: Text(
                    'SIZE',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
                DataColumn(
                  label: Text(
                    'THROUGHPUT',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
                DataColumn(
                  label: Text(
                    'PEERS',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
                DataColumn(
                  label: Text(
                    'ETA',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
                DataColumn(
                  label: Text(
                    'ACTIONS',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
              ],
              rows: queueItems.map((item) {
                final group = _extractGroup(item.title);
                return DataRow(
                  cells: [
                    DataCell(
                      SizedBox(
                        width: 320,
                        child: Row(
                          children: [
                            const Icon(
                              Icons.sync_rounded,
                              size: 18,
                              color: ObsidianColors.tertiary,
                            ),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                mainAxisAlignment: MainAxisAlignment.center,
                                children: [
                                  Row(
                                    children: [
                                      if (group != null) ...[
                                        Container(
                                          padding: const EdgeInsets.symmetric(
                                            horizontal: 4,
                                            vertical: 1,
                                          ),
                                          decoration: BoxDecoration(
                                            color: ObsidianColors
                                                .surfaceContainerHighest,
                                            borderRadius: BorderRadius.circular(
                                              3,
                                            ),
                                          ),
                                          child: Text(
                                            group,
                                            style:
                                                ObsidianTypography.labelBadge(
                                                  color: ObsidianColors.primary,
                                                  fontSize: 9,
                                                ),
                                          ),
                                        ),
                                        const SizedBox(width: 6),
                                      ],
                                      Expanded(
                                        child: Text(
                                          item.title,
                                          style: ObsidianTypography.titleSm(
                                            fontSize: 12,
                                          ),
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis,
                                        ),
                                      ),
                                    ],
                                  ),
                                  Text(
                                    'Target: ${state.locationsReport?['library']?['path'] ?? 'Library'}',
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
                    ),
                    DataCell(
                      SizedBox(
                        width: 140,
                        child: Column(
                          mainAxisAlignment: MainAxisAlignment.center,
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            Row(
                              mainAxisAlignment: MainAxisAlignment.spaceBetween,
                              children: [
                                Text(
                                  '${(item.progress * 100).toStringAsFixed(1)}%',
                                  style: ObsidianTypography.monoMetric(
                                    color: ObsidianColors.tertiary,
                                    fontSize: 11,
                                  ),
                                ),
                                Text(
                                  item.formattedSize,
                                  style: ObsidianTypography.monoCode(
                                    color: ObsidianColors.outline,
                                    fontSize: 10,
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 4),
                            ClipRRect(
                              borderRadius: BorderRadius.circular(3),
                              child: LinearProgressIndicator(
                                value: item.progress,
                                minHeight: 5,
                                backgroundColor:
                                    ObsidianColors.surfaceContainerHighest,
                                valueColor: const AlwaysStoppedAnimation(
                                  ObsidianColors.tertiary,
                                ),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                    DataCell(
                      Text(
                        item.formattedSize,
                        style: ObsidianTypography.monoMetric(
                          color: ObsidianColors.onSurfaceVariant,
                          fontSize: 11,
                        ),
                      ),
                    ),
                    DataCell(
                      Text(
                        item.downloadSpeed,
                        style: ObsidianTypography.monoMetric(
                          color: ObsidianColors.tertiary,
                          fontSize: 11,
                        ),
                      ),
                    ),
                    DataCell(
                      Text(
                        '${item.seeders} (${item.leechers})',
                        style: ObsidianTypography.monoMetric(
                          color: ObsidianColors.onSurfaceVariant,
                          fontSize: 11,
                        ),
                      ),
                    ),
                    DataCell(
                      Text(
                        item.eta,
                        style: ObsidianTypography.monoMetric(
                          color: ObsidianColors.primary,
                          fontSize: 11,
                        ),
                      ),
                    ),
                    DataCell(
                      Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          IconButton(
                            icon: const Icon(
                              Icons.restart_alt_rounded,
                              size: 16,
                              color: ObsidianColors.outline,
                            ),
                            tooltip: 'Force Check',
                            onPressed: () {},
                          ),
                          IconButton(
                            icon: const Icon(
                              Icons.pause_rounded,
                              size: 16,
                              color: ObsidianColors.outline,
                            ),
                            tooltip: 'Pause',
                            onPressed: () {},
                          ),
                        ],
                      ),
                    ),
                  ],
                );
              }).toList(),
            ),
          ),
        ],
      ),
    );
  }
}
