import 'dart:io';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/activity_item.dart';
import '../models/library_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/state_views.dart';

class DashboardPage extends StatefulWidget {
  const DashboardPage({super.key});

  @override
  State<DashboardPage> createState() => _DashboardPageState();
}

class _DashboardPageState extends State<DashboardPage> {
  final _searchFilterController = TextEditingController();

  @override
  void dispose() {
    _searchFilterController.dispose();
    super.dispose();
  }

  void _openFileExplorer(String path) {
    if (!Platform.environment.containsKey('FLUTTER_TEST')) {
      Process.run('xdg-open', [path]);
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    if (state.isLoadingStatus && state.queue.isEmpty && state.shows.isEmpty && state.coreError == null) {
      return const Center(
        child: LoadingStateView(message: 'Querying system telemetry from genrl core...'),
      );
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (state.coreError != null) ...[
            ErrorStateView(
              title: 'Core Engine Unreachable',
              message: state.coreError!,
              onRetry: () => state.refreshAll(),
              retryLabel: 'Retry Connection',
            ),
            const SizedBox(height: 16),
          ],

          // 1. Operational Command & Action Bar
          _buildOperationalBar(context, state),
          const SizedBox(height: 12),

          // 2. Health & State Pulse Bar
          _buildHealthPulseBar(context, state),
          const SizedBox(height: 16),

          // 3. KPI Metric Strip (4 Cards)
          _buildKpiMetrics(context, state),
          const SizedBox(height: 20),

          // 4. Daemon Pipeline & Active Ingestion Table
          _buildPipelineTable(context, state),
          const SizedBox(height: 24),

          // 5. Airing Today & Monitored Next Up (2:3 Poster Grid)
          _buildAiringPosterGrid(context, state),
          const SizedBox(height: 24),

          // 6. Bottom Split Grid: Ingestions Ledger + Tracker Matrix
          _buildBottomSplitGrid(context, state),
          const SizedBox(height: 16),
        ],
      ),
    );
  }

  Widget _buildOperationalBar(BuildContext context, StateProvider state) {
    final status = state.status;

    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Wrap(
        alignment: WrapAlignment.spaceBetween,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: 12,
        runSpacing: 10,
        children: [
          // Search & Quick Jump Box
          ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 420, minWidth: 260),
            child: Container(
              height: 38,
              padding: const EdgeInsets.symmetric(horizontal: 10),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLowest,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: ObsidianColors.border),
              ),
              child: Row(
                children: [
                  const Icon(Icons.travel_explore_rounded, size: 18, color: ObsidianColors.outline),
                  const SizedBox(width: 8),
                  Expanded(
                    child: TextField(
                      controller: _searchFilterController,
                      style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
                      decoration: InputDecoration(
                        hintText: 'Quick jump or filter anime... (Ctrl+K)',
                        hintStyle: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                        border: InputBorder.none,
                        isDense: true,
                      ),
                      onSubmitted: (val) {
                        if (val.trim().isNotEmpty) {
                          state.selectPage(4); // Search page
                        }
                      },
                    ),
                  ),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 2),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerHighest,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text('Ctrl+K', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 10)),
                  ),
                ],
              ),
            ),
          ),

          // qBit Link & Action Buttons
          Wrap(
            spacing: 8,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: 7,
                      height: 7,
                      decoration: BoxDecoration(
                        color: status.qbitConnected ? ObsidianColors.tertiary : ObsidianColors.error,
                        shape: BoxShape.circle,
                      ),
                    ),
                    const SizedBox(width: 8),
                    Text(
                      'qBittorrent ${status.qbitVersion} :: ${status.qbitConnected ? "IPC Linked" : "Offline"}',
                      style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 11),
                    ),
                  ],
                ),
              ),

              OutlinedButton.icon(
                onPressed: state.isSyncing ? null : () => state.forceSyncMal(),
                icon: state.isSyncing
                    ? const SizedBox(width: 14, height: 14, child: CircularProgressIndicator(strokeWidth: 2, color: ObsidianColors.primary))
                    : const Icon(Icons.sync_rounded, size: 16, color: ObsidianColors.primary),
                label: Text('Force Sync RSS', style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface)),
                style: OutlinedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainer,
                  side: const BorderSide(color: ObsidianColors.border),
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
              ),

              OutlinedButton.icon(
                onPressed: state.isScanning ? null : () => state.manualDiskScan(),
                icon: state.isScanning
                    ? const SizedBox(width: 14, height: 14, child: CircularProgressIndicator(strokeWidth: 2, color: ObsidianColors.tertiary))
                    : const Icon(Icons.folder_special_rounded, size: 16, color: ObsidianColors.tertiary),
                label: Text('Manual Disk Scan', style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface)),
                style: OutlinedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainer,
                  side: const BorderSide(color: ObsidianColors.border),
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
              ),

              ElevatedButton.icon(
                onPressed: () => state.selectPage(4),
                icon: const Icon(Icons.bolt_rounded, size: 16),
                label: Text('Grab Next Available', style: ObsidianTypography.titleSm(color: ObsidianColors.onPrimaryContainer, fontSize: 12)),
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.primaryContainer,
                  foregroundColor: ObsidianColors.onPrimaryContainer,
                  elevation: 0,
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildHealthPulseBar(BuildContext context, StateProvider state) {
    final activeTorrents = state.status.activeTorrents > 0
        ? state.status.activeTorrents
        : state.activeQueueCount;
    final monitoredShows = state.status.totalShows > 0
        ? state.status.totalShows
        : state.shows.length;
    final nextShow = state.shows.isNotEmpty ? state.shows.first.title : '-';

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Expanded(
            child: Wrap(
              crossAxisAlignment: WrapCrossAlignment.center,
              spacing: 12,
              runSpacing: 4,
              children: [
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(Icons.download_rounded, size: 14, color: ObsidianColors.tertiary),
                    const SizedBox(width: 4),
                    Text(
                      '$activeTorrents Active Torrents',
                      style: ObsidianTypography.monoMetric(color: ObsidianColors.onSurface, fontSize: 12),
                    ),
                  ],
                ),
                Text('/', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(Icons.bookmark_added_rounded, size: 14, color: ObsidianColors.primary),
                    const SizedBox(width: 4),
                    Text(
                      '$monitoredShows Monitored Shows',
                      style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 12),
                    ),
                  ],
                ),
                Text('/', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(Icons.update_rounded, size: 14, color: ObsidianColors.secondary),
                    const SizedBox(width: 4),
                    RichText(
                      text: TextSpan(
                        style: ObsidianTypography.monoCode(fontSize: 12),
                        children: [
                          const TextSpan(text: 'Next Release: ', style: TextStyle(color: ObsidianColors.primaryFixedDim)),
                          TextSpan(text: nextShow, style: const TextStyle(color: ObsidianColors.onSurface, fontWeight: FontWeight.bold)),
                          const TextSpan(text: ' (~38m)', style: TextStyle(color: ObsidianColors.outline)),
                        ],
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('CRON: ACTIVE', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline, fontSize: 10)),
              ),
              const SizedBox(width: 6),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('PARSE: NYAA+AB', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary, fontSize: 10)),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildKpiMetrics(BuildContext context, StateProvider state) {
    final status = state.status;
    final totalShows = state.shows.length;
    final totalEpisodes = state.shows.fold<int>(0, (sum, show) => sum + show.totalEpisodes);
    final activeQueue = state.activeQueueCount;
    final dlSpeed = status.downloadSpeedFormatted;

    return LayoutBuilder(
      builder: (context, constraints) {
        final cardWidth = constraints.maxWidth > 1150
            ? (constraints.maxWidth - 36) / 4
            : constraints.maxWidth > 700
                ? (constraints.maxWidth - 12) / 2
                : constraints.maxWidth;
        final isCompact = constraints.maxWidth <= 700;

        return Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            // Card 1: Pipeline Flow
            _KpiCard(
              width: isCompact ? constraints.maxWidth : cardWidth,
              label: 'PIPELINE FLOW',
              icon: Icons.cloud_download_rounded,
              iconColor: ObsidianColors.primary,
              headline: '$activeQueue',
              subhead: 'Torrents Active',
              detail: '$dlSpeed • ETA ~14m 12s',
              progress: 0.68,
              progressColor: ObsidianColors.primary,
            ),

            // Card 2: Library Index
            _KpiCard(
              width: isCompact ? constraints.maxWidth : cardWidth,
              label: 'LIBRARY INDEX',
              icon: Icons.video_library_rounded,
              iconColor: ObsidianColors.tertiary,
              headline: totalShows > 0 ? '$totalShows' : '1,428',
              subhead: 'Series',
              detail: '$totalEpisodes Ep • 18.2 TB on /mnt/anime',
              progress: 0.82,
              progressColor: ObsidianColors.tertiary,
            ),

            // Card 3: Missing Catalog
            _KpiCard(
              width: isCompact ? constraints.maxWidth : cardWidth,
              label: 'MISSING CATALOG',
              icon: Icons.rule_folder_rounded,
              iconColor: ObsidianColors.secondary,
              headline: '19',
              subhead: 'Episodes Unmet',
              subheadColor: ObsidianColors.error,
              detail: '7 Ready in monitored RSS index',
              progress: 0.38,
              progressColor: ObsidianColors.secondaryContainer,
            ),

            // Card 4: Current Cycle
            _KpiCard(
              width: isCompact ? constraints.maxWidth : cardWidth,
              label: 'CURRENT CYCLE',
              icon: Icons.calendar_month_rounded,
              iconColor: ObsidianColors.primary,
              headline: 'Spring 2025',
              headlineSize: 22,
              subhead: '16 Tracked',
              detail: '6 Airing in next 48h',
              progress: 0.90,
              progressColor: ObsidianColors.primaryContainer,
            ),
          ],
        );
      },
    );
  }

  Widget _buildPipelineTable(BuildContext context, StateProvider state) {
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
                const Icon(Icons.downloading_rounded, size: 20, color: ObsidianColors.primary),
                const SizedBox(width: 8),
                Text('Daemon Pipeline & Active Ingestion', style: ObsidianTypography.headlineMd()),
                const SizedBox(width: 10),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text('ACTIVE DOWNLOAD TRANSFERS', style: ObsidianTypography.labelBadge(color: ObsidianColors.primary)),
                ),
              ],
            ),
            const SizedBox(height: 20),
            Center(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Text(
                  'Queue is empty. Automation is watching for newly aired releases.',
                  style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
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
                  const Icon(Icons.downloading_rounded, size: 20, color: ObsidianColors.primary),
                  Text('Daemon Pipeline & Active Ingestion', style: ObsidianTypography.headlineMd()),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerHighest,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text('ACTIVE DOWNLOAD TRANSFERS', style: ObsidianTypography.labelBadge(color: ObsidianColors.primary)),
                  ),
                ],
              ),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  OutlinedButton.icon(
                    onPressed: () => state.toggleAutomation(),
                    icon: Icon(state.status.automationOn ? Icons.pause_rounded : Icons.play_arrow_rounded, size: 14),
                    label: Text(state.status.automationOn ? 'Pause All' : 'Resume', style: ObsidianTypography.monoCode(fontSize: 11)),
                    style: OutlinedButton.styleFrom(
                      side: const BorderSide(color: ObsidianColors.border),
                      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
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
              headingRowColor: WidgetStateProperty.all(ObsidianColors.surfaceContainerLowest),
              columns: [
                DataColumn(label: Text('PAYLOAD / SERIES', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                DataColumn(label: Text('PROGRESS', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                DataColumn(label: Text('SIZE', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                DataColumn(label: Text('THROUGHPUT', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                DataColumn(label: Text('PEERS', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                DataColumn(label: Text('ETA', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                DataColumn(label: Text('ACTIONS', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
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
                            const Icon(Icons.sync_rounded, size: 18, color: ObsidianColors.tertiary),
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
                                          padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                                          decoration: BoxDecoration(
                                            color: ObsidianColors.surfaceContainerHighest,
                                            borderRadius: BorderRadius.circular(3),
                                          ),
                                          child: Text(group, style: ObsidianTypography.labelBadge(color: ObsidianColors.primary, fontSize: 9)),
                                        ),
                                        const SizedBox(width: 6),
                                      ],
                                      Expanded(
                                        child: Text(
                                          item.title,
                                          style: ObsidianTypography.titleSm(fontSize: 12),
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis,
                                        ),
                                      ),
                                    ],
                                  ),
                                  Text(
                                    'Target: /mnt/anime/TV/...',
                                    style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10),
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
                                Text('${(item.progress * 100).toStringAsFixed(1)}%', style: ObsidianTypography.monoMetric(color: ObsidianColors.tertiary, fontSize: 11)),
                                Text(item.formattedSize, style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10)),
                              ],
                            ),
                            const SizedBox(height: 4),
                            ClipRRect(
                              borderRadius: BorderRadius.circular(3),
                              child: LinearProgressIndicator(
                                value: item.progress,
                                minHeight: 5,
                                backgroundColor: ObsidianColors.surfaceContainerHighest,
                                valueColor: const AlwaysStoppedAnimation(ObsidianColors.tertiary),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                    DataCell(Text(item.formattedSize, style: ObsidianTypography.monoMetric(color: ObsidianColors.onSurfaceVariant, fontSize: 11))),
                    DataCell(Text(item.downloadSpeed, style: ObsidianTypography.monoMetric(color: ObsidianColors.tertiary, fontSize: 11))),
                    DataCell(Text('${item.seeders} (${item.leechers})', style: ObsidianTypography.monoMetric(color: ObsidianColors.onSurfaceVariant, fontSize: 11))),
                    DataCell(Text(item.eta, style: ObsidianTypography.monoMetric(color: ObsidianColors.primary, fontSize: 11))),
                    DataCell(
                      Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          IconButton(
                            icon: const Icon(Icons.restart_alt_rounded, size: 16, color: ObsidianColors.outline),
                            tooltip: 'Force Check',
                            onPressed: () {},
                          ),
                          IconButton(
                            icon: const Icon(Icons.pause_rounded, size: 16, color: ObsidianColors.outline),
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

  Widget _buildAiringPosterGrid(BuildContext context, StateProvider state) {
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
                const Icon(Icons.live_tv_rounded, size: 20, color: ObsidianColors.primary),
                const SizedBox(width: 8),
                Text('Airing Today & Monitored Next Up', style: ObsidianTypography.headlineMd()),
              ],
            ),
            const SizedBox(height: 20),
            Center(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Text(
                  'No organized shows discovered yet. Run a disk scan or connect MAL to populate.',
                  style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
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
                const Icon(Icons.live_tv_rounded, size: 20, color: ObsidianColors.primary),
                const SizedBox(width: 8),
                Text('Airing Today & Monitored Next Up', style: ObsidianTypography.headlineMd()),
              ],
            ),
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  'Auto-snatch: ',
                  style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 11),
                ),
                Text(
                  'ENABLED',
                  style: ObsidianTypography.monoCode(color: ObsidianColors.tertiary, fontSize: 11, fontWeight: FontWeight.bold),
                ),
                const SizedBox(width: 14),
                InkWell(
                  onTap: () => state.selectPage(3), // Library
                  child: Row(
                    children: [
                      Text('Full Catalog', style: ObsidianTypography.bodySm(color: ObsidianColors.primary)),
                      const Icon(Icons.arrow_forward_rounded, size: 14, color: ObsidianColors.primary),
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
            final itemWidth = (constraints.maxWidth - (columns - 1) * 12) / columns;

            return Wrap(
              spacing: 12,
              runSpacing: 12,
              children: shows.map((show) {
                return SizedBox(
                  width: itemWidth,
                  child: _DashboardPosterCard(
                    show: show,
                    onTap: () => state.openShowDetail(show),
                    onOpenFolder: () => _openFileExplorer(show.folderPath ?? '/mnt/anime/TV/${show.title}'),
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

  Widget _buildBottomSplitGrid(BuildContext context, StateProvider state) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final isStacked = constraints.maxWidth < 900;
        return Flex(
          direction: isStacked ? Axis.vertical : Axis.horizontal,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Left 2 Cols: Recent Library Ingestions & Re-links
            Expanded(
              flex: isStacked ? 0 : 2,
              child: Container(
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
                      spacing: 8,
                      runSpacing: 4,
                      children: [
                        Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            const Icon(Icons.assignment_turned_in_rounded, size: 18, color: ObsidianColors.primary),
                            const SizedBox(width: 8),
                            Text('Recent Library Ingestions & Re-links', style: ObsidianTypography.headlineMd(fontSize: 16)),
                          ],
                        ),
                        Text('Hardlink Mode: ON', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 11)),
                      ],
                    ),
                    const SizedBox(height: 12),
                    if (state.events.isNotEmpty)
                      ...state.events.take(3).map((act) => Padding(
                            padding: const EdgeInsets.only(bottom: 8.0),
                            child: _buildFeedRow(
                              dotColor: act.level == ActivityLevel.error
                                  ? ObsidianColors.error
                                  : (act.level == ActivityLevel.warn ? ObsidianColors.warning : ObsidianColors.primary),
                              title: act.message,
                              group: act.jobId != null ? 'Job #${act.jobId}' : (act.malId != null ? 'Show #${act.malId}' : 'Core'),
                              target: 'Event #${act.id} recorded in events store',
                              time: act.formattedTime,
                            ),
                          ))
                    else
                      Padding(
                        padding: const EdgeInsets.symmetric(vertical: 16.0),
                        child: Center(
                          child: Text(
                            'No recent ingestions or disk links yet',
                            style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                          ),
                        ),
                      ),
                  ],
                ),
              ),
            ),
            if (!isStacked) const SizedBox(width: 14) else const SizedBox(height: 14),

            // Right 1 Col: Tracker / RSS Matrix
            Expanded(
              flex: isStacked ? 0 : 1,
              child: Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLow,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Expanded(
                          child: Row(
                            children: [
                              const Icon(Icons.sensors_rounded, size: 18, color: ObsidianColors.tertiary),
                              const SizedBox(width: 8),
                              Expanded(
                                child: Text('Tracker / RSS Matrix', style: ObsidianTypography.headlineMd(fontSize: 16), overflow: TextOverflow.ellipsis),
                              ),
                            ],
                          ),
                        ),
                        IconButton(
                          icon: const Icon(Icons.refresh_rounded, size: 16, color: ObsidianColors.outline),
                          onPressed: () {},
                          padding: EdgeInsets.zero,
                          constraints: const BoxConstraints(),
                        ),
                      ],
                    ),
                    const SizedBox(height: 12),
                    _buildTrackerRow('Nyaa.si (Prowlarr)', '42 ms', 'HTTP 200', ObsidianColors.tertiary),
                    const SizedBox(height: 6),
                    _buildTrackerRow('AnimeBytes (Passkey)', '110 ms', 'AUTH OK', ObsidianColors.tertiary),
                    const SizedBox(height: 6),
                    _buildTrackerRow('Tokyo Toshokan', '85 ms', 'HTTP 200', ObsidianColors.tertiary),
                    const SizedBox(height: 12),
                    Container(
                      padding: const EdgeInsets.all(8),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainer.withOpacity(0.6),
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text('Poll Interval: 15 mins', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 10)),
                                Text('Sync Queue: 0 pending', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10)),
                              ],
                            ),
                          ),
                          Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              const Icon(Icons.shield_rounded, size: 16, color: ObsidianColors.primary),
                              const SizedBox(width: 4),
                              Text('100% SLA', style: ObsidianTypography.monoMetric(color: ObsidianColors.primary, fontSize: 11, fontWeight: FontWeight.bold)),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        );
      },
    );
  }

  Widget _buildFeedRow({
    required Color dotColor,
    required String title,
    required String group,
    required String target,
    required String time,
  }) {
    return Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer.withOpacity(0.4),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        children: [
          Container(
            width: 6,
            height: 6,
            decoration: BoxDecoration(color: dotColor, shape: BoxShape.circle),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        title,
                        style: ObsidianTypography.titleSm(fontSize: 12),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    const SizedBox(width: 6),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHighest,
                        borderRadius: BorderRadius.circular(3),
                      ),
                      child: Text(group, style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary, fontSize: 9)),
                    ),
                  ],
                ),
                Text(
                  target,
                  style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          const SizedBox(width: 8),
          Text(time, style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 10)),
        ],
      ),
    );
  }

  Widget _buildTrackerRow(String name, String ping, String statusBadge, Color dotColor) {
    return Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Expanded(
            child: Row(
              children: [
                Container(width: 6, height: 6, decoration: BoxDecoration(color: dotColor, shape: BoxShape.circle)),
                const SizedBox(width: 8),
                Expanded(child: Text(name, style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface, fontSize: 11), overflow: TextOverflow.ellipsis)),
              ],
            ),
          ),
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(ping, style: ObsidianTypography.monoMetric(color: ObsidianColors.tertiary, fontSize: 11)),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1.5),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(statusBadge, style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurfaceVariant, fontSize: 9)),
              ),
            ],
          ),
        ],
      ),
    );
  }

  String? _extractGroup(String title) {
    if (title.startsWith('[') && title.contains(']')) {
      final end = title.indexOf(']');
      return title.substring(1, end);
    }
    return null;
  }
}

class _KpiCard extends StatelessWidget {
  final double width;
  final String label;
  final IconData icon;
  final Color iconColor;
  final String headline;
  final double? headlineSize;
  final String subhead;
  final Color? subheadColor;
  final String detail;
  final double progress;
  final Color progressColor;

  const _KpiCard({
    required this.width,
    required this.label,
    required this.icon,
    required this.iconColor,
    required this.headline,
    this.headlineSize,
    required this.subhead,
    this.subheadColor,
    required this.detail,
    required this.progress,
    required this.progressColor,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      width: width,
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(label, style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurfaceVariant, fontSize: 10)),
              Icon(icon, size: 18, color: iconColor),
            ],
          ),
          const SizedBox(height: 8),
          Wrap(
            crossAxisAlignment: WrapCrossAlignment.end,
            spacing: 6,
            runSpacing: 2,
            children: [
              Text(headline, style: ObsidianTypography.headlineXl(fontSize: headlineSize ?? 28)),
              Text(
                subhead,
                style: ObsidianTypography.titleSm(color: subheadColor ?? ObsidianColors.tertiary, fontSize: 12),
                overflow: TextOverflow.ellipsis,
              ),
            ],
          ),
          const SizedBox(height: 4),
          Text(detail, style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 10), maxLines: 1, overflow: TextOverflow.ellipsis),
          const SizedBox(height: 10),
          ClipRRect(
            borderRadius: BorderRadius.circular(3),
            child: LinearProgressIndicator(
              value: progress,
              minHeight: 4,
              backgroundColor: ObsidianColors.surfaceContainerHighest,
              valueColor: AlwaysStoppedAnimation(progressColor),
            ),
          ),
        ],
      ),
    );
  }
}

class _DashboardPosterCard extends StatefulWidget {
  final LibraryShow show;
  final VoidCallback onTap;
  final VoidCallback onOpenFolder;
  final VoidCallback onRefreshMeta;
  final VoidCallback onSearch;

  const _DashboardPosterCard({
    required this.show,
    required this.onTap,
    required this.onOpenFolder,
    required this.onRefreshMeta,
    required this.onSearch,
  });

  @override
  State<_DashboardPosterCard> createState() => _DashboardPosterCardState();
}

class _DashboardPosterCardState extends State<_DashboardPosterCard> {
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
              color: _isHovered ? ObsidianColors.primary.withOpacity(0.6) : ObsidianColors.border,
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
                        errorBuilder: (_, __, ___) => _buildPosterPlaceholder(show),
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
                        padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 2),
                        decoration: BoxDecoration(
                          color: ObsidianColors.surfaceContainerLowest.withOpacity(0.85),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          show.episodesDownloaded == show.episodesTotal ? 'Complete' : '1080p Remux',
                          style: ObsidianTypography.labelBadge(
                            color: show.episodesDownloaded == show.episodesTotal ? ObsidianColors.success : ObsidianColors.tertiary,
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
                          color: ObsidianColors.surfaceContainerLowest.withOpacity(0.85),
                          shape: BoxShape.circle,
                        ),
                        child: const Icon(Icons.bookmark_rounded, size: 14, color: ObsidianColors.primary),
                      ),
                    ),

                    // Hover Quick Action Micro-Dock
                    if (_isHovered)
                      Positioned(
                        top: 36,
                        left: 10,
                        right: 10,
                        child: Container(
                          padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainerLowest.withOpacity(0.92),
                            borderRadius: BorderRadius.circular(6),
                            border: Border.all(color: ObsidianColors.border),
                          ),
                          child: Row(
                            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                            children: [
                              InkWell(
                                onTap: widget.onSearch,
                                child: const Icon(Icons.search_rounded, size: 16, color: ObsidianColors.onSurface),
                              ),
                              InkWell(
                                onTap: widget.onRefreshMeta,
                                child: const Icon(Icons.refresh_rounded, size: 16, color: ObsidianColors.onSurface),
                              ),
                              InkWell(
                                onTap: widget.onOpenFolder,
                                child: const Icon(Icons.folder_open_rounded, size: 16, color: ObsidianColors.onSurface),
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
                                style: ObsidianTypography.monoMetric(color: ObsidianColors.tertiary, fontSize: 10),
                              ),
                              Text('★ ${show.score.toStringAsFixed(1)}', style: ObsidianTypography.monoMetric(color: ObsidianColors.primary, fontSize: 10, fontWeight: FontWeight.bold)),
                            ],
                          ),
                          const SizedBox(height: 2),
                          Text(
                            show.title,
                            style: ObsidianTypography.titleSm(color: Colors.white, fontSize: 12),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          Text(
                            show.status == 'Airing' ? 'Airs weekly' : 'Complete',
                            style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 9),
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
