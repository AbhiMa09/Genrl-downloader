import 'dart:io';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/queue_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/state_views.dart';

class QueuePage extends StatefulWidget {
  const QueuePage({super.key});

  @override
  State<QueuePage> createState() => _QueuePageState();
}

class _QueuePageState extends State<QueuePage> {
  String _selectedFilter = 'all';
  final _searchController = TextEditingController();
  QueueItem? _selectedItem;

  @override
  void dispose() {
    _searchController.dispose();
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

    if (state.coreError != null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24.0),
          child: ErrorStateView(
            title: 'Queue Query Error',
            message: state.coreError!,
            onRetry: () => state.refreshAll(),
            retryLabel: 'Retry Queue',
          ),
        ),
      );
    }

    if (state.queue.isEmpty) {
      return const Center(
        child: Padding(
          padding: EdgeInsets.all(24.0),
          child: EmptyStateView(
            title: 'No Transfers In Queue',
            description: 'Active downloads and jobs will appear here live as they are processed.',
          ),
        ),
      );
    }

    final queueItems = state.queue;

    // Filter items
    final query = _searchController.text.trim().toLowerCase();
    final filteredItems = queueItems.where((item) {
      if (_selectedFilter != 'all') {
        if (_selectedFilter == 'downloading' && item.state != QueueState.downloading) return false;
        if (_selectedFilter == 'finalizing' && item.state != QueueState.finalizing && item.state != QueueState.completed) return false;
        if (_selectedFilter == 'blocked' && item.state != QueueState.blocked) return false;
        if (_selectedFilter == 'failed' && item.state != QueueState.stalled) return false;
        if (_selectedFilter == 'searching' && item.state != QueueState.queued) return false;
        if (_selectedFilter == 'needs_input' && item.seeders > 2 && item.state != QueueState.blocked) return false;
      }
      if (query.isNotEmpty) {
        if (!item.title.toLowerCase().contains(query)) return false;
      }
      return true;
    }).toList();

    final activeItem = _selectedItem ?? (filteredItems.isNotEmpty ? filteredItems.first : null);

    return SingleChildScrollView(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // 1. Queue Header & Command Toolbar
          _buildHeaderToolbar(context, state, queueItems.length),

          // 2. Filter Tabs & Quick Search Bar
          _buildFilterTabs(queueItems),

          // 3. Main Work Area: Dual Pane Table + Inspector
          Padding(
            padding: const EdgeInsets.all(16),
            child: LayoutBuilder(
              builder: (context, constraints) {
                final isStacked = constraints.maxWidth < 1000;

                return Flex(
                  direction: isStacked ? Axis.vertical : Axis.horizontal,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // Left: Dense Queue Table
                    Expanded(
                      flex: isStacked ? 0 : 7,
                      child: _buildQueueTable(filteredItems, activeItem),
                    ),
                    if (!isStacked) const SizedBox(width: 16) else const SizedBox(height: 16),

                    // Right: Job Details & Decision Inspector
                    if (activeItem != null)
                      SizedBox(
                        width: isStacked ? constraints.maxWidth : 380,
                        child: _buildInspectorDrawer(activeItem),
                      ),
                  ],
                );
              },
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildHeaderToolbar(BuildContext context, StateProvider state, int count) {
    final status = state.status;

    return Container(
      padding: const EdgeInsets.fromLTRB(24, 16, 24, 12),
      decoration: const BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        border: Border(bottom: BorderSide(color: ObsidianColors.border)),
      ),
      child: Wrap(
        alignment: WrapAlignment.spaceBetween,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: 16,
        runSpacing: 10,
        children: [
          // Left: Title & Quick Swarm Telemetry
          Wrap(
            crossAxisAlignment: WrapCrossAlignment.center,
            spacing: 12,
            runSpacing: 6,
            children: [
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text('Ingestion Queue', style: ObsidianTypography.headlineMd()),
                  const SizedBox(width: 8),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainer,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text('$count active jobs', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 11)),
                  ),
                ],
              ),
              Container(width: 1, height: 14, color: ObsidianColors.border),
              Wrap(
                spacing: 8,
                children: [
                  _buildPill('DL: ', status.downloadSpeedFormatted, ObsidianColors.primary),
                  _buildPill('UL: ', status.uploadSpeedFormatted, ObsidianColors.tertiary),
                  _buildPill('Free Space: ', '4.58 TB', ObsidianColors.onSurfaceVariant),
                ],
              ),
            ],
          ),

          // Right: Global Actions
          Wrap(
            spacing: 8,
            runSpacing: 6,
            children: [
              OutlinedButton.icon(
                onPressed: () => state.toggleAutomation(),
                icon: Icon(state.status.automationOn ? Icons.pause_rounded : Icons.play_arrow_rounded, size: 14, color: ObsidianColors.outline),
                label: Text(state.status.automationOn ? 'Pause Queue' : 'Resume Queue', style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface, fontSize: 11)),
                style: OutlinedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainer,
                  side: const BorderSide(color: ObsidianColors.border),
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
              ),

              OutlinedButton.icon(
                onPressed: () => state.refreshAll(),
                icon: const Icon(Icons.restart_alt_rounded, size: 14, color: ObsidianColors.tertiary),
                label: Text('Retry Failed', style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface, fontSize: 11)),
                style: OutlinedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainer,
                  side: const BorderSide(color: ObsidianColors.border),
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
              ),

              ElevatedButton.icon(
                onPressed: () => state.selectPage(4),
                icon: const Icon(Icons.add_rounded, size: 16),
                label: Text('Manual Magnet', style: ObsidianTypography.titleSm(color: ObsidianColors.onPrimary, fontSize: 12)),
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.primary,
                  foregroundColor: ObsidianColors.onPrimary,
                  elevation: 0,
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildFilterTabs(List<QueueItem> items) {
    final dlCount = items.where((i) => i.state == QueueState.downloading).length;
    final finalCount = items.where((i) => i.state == QueueState.finalizing || i.state == QueueState.completed).length;
    final blockedCount = items.where((i) => i.state == QueueState.blocked).length;
    final failedCount = items.where((i) => i.state == QueueState.stalled).length;
    final searchCount = items.where((i) => i.state == QueueState.queued).length;

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 10),
      decoration: const BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        border: Border(bottom: BorderSide(color: ObsidianColors.border)),
      ),
      child: Wrap(
        alignment: WrapAlignment.spaceBetween,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: 16,
        runSpacing: 10,
        children: [
          // Filter Tabs
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                _buildTab('all', 'All', '${items.length}', ObsidianColors.primary),
                const SizedBox(width: 6),
                _buildTab('searching', 'Searching', '$searchCount', ObsidianColors.tertiary),
                const SizedBox(width: 6),
                _buildTab('downloading', 'Downloading', '$dlCount', ObsidianColors.primary),
                const SizedBox(width: 6),
                _buildTab('finalizing', 'Finalizing', '$finalCount', ObsidianColors.onSurface),
                const SizedBox(width: 6),
                _buildTab('blocked', 'Blocked', '$blockedCount', ObsidianColors.error),
                const SizedBox(width: 6),
                _buildTab('failed', 'Failed', '$failedCount', ObsidianColors.error),
                const SizedBox(width: 6),
                _buildTab('needs_input', 'Needs Input', '2', ObsidianColors.secondary),
              ],
            ),
          ),

          // Search Input
          Container(
            width: 280,
            height: 36,
            padding: const EdgeInsets.symmetric(horizontal: 10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: ObsidianColors.border),
            ),
            child: Row(
              children: [
                const Icon(Icons.filter_list_rounded, size: 16, color: ObsidianColors.outline),
                const SizedBox(width: 8),
                Expanded(
                  child: TextField(
                    controller: _searchController,
                    style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
                    decoration: InputDecoration(
                      hintText: 'Filter title, release group, hash...',
                      hintStyle: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                      border: InputBorder.none,
                      isDense: true,
                    ),
                    onChanged: (_) => setState(() {}),
                  ),
                ),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1.5),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHigh,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text('Ctrl+F', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 9)),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildTab(String key, String label, String badge, Color badgeColor) {
    final isSelected = _selectedFilter == key;
    return InkWell(
      onTap: () => setState(() => _selectedFilter = key),
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: isSelected ? ObsidianColors.primary : ObsidianColors.surfaceContainer,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isSelected ? ObsidianColors.primary : ObsidianColors.border,
            width: 0.5,
          ),
        ),
        child: Text(
          '$label ($badge)',
          style: ObsidianTypography.monoCode(
            color: isSelected ? ObsidianColors.onPrimary : ObsidianColors.onSurfaceVariant,
            fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
            fontSize: 11,
          ),
        ),
      ),
    );
  }

  Widget _buildQueueTable(List<QueueItem> items, QueueItem? activeItem) {
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
                Expanded(flex: 5, child: Text('ANIME & EPISODE', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                Expanded(flex: 3, child: Text('STATE / PHASE', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                Expanded(flex: 2, child: Text('SEEDERS', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                Expanded(flex: 3, child: Text('PROGRESS & SPEED', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                Expanded(flex: 2, child: Text('ETA', textAlign: TextAlign.right, style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                const SizedBox(width: 60, child: Text('ACTIONS', textAlign: TextAlign.center, style: TextStyle(fontSize: 10, color: ObsidianColors.outline))),
              ],
            ),
          ),

          // Scrollable Queue Rows
          ListView.separated(
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            itemCount: items.length,
            separatorBuilder: (_, __) => const Divider(height: 1, color: ObsidianColors.border),
            itemBuilder: (context, index) {
              final item = items[index];
              final isSelected = activeItem?.id == item.id;

              return InkWell(
                onTap: () => setState(() => _selectedItem = item),
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                  decoration: BoxDecoration(
                    color: isSelected
                        ? ObsidianColors.surfaceContainer.withOpacity(0.9)
                        : Colors.transparent,
                    border: isSelected
                        ? const Border(left: BorderSide(color: ObsidianColors.primary, width: 3))
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
                              child: const Icon(Icons.movie_filter_rounded, size: 16, color: ObsidianColors.outline),
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
                                            color: isSelected ? ObsidianColors.primary : ObsidianColors.onSurface,
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
                                    style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10),
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
                                  color: item.seeders > 10 ? ObsidianColors.tertiary : ObsidianColors.error,
                                  fontWeight: FontWeight.bold,
                                ),
                              ),
                              TextSpan(text: ' (${item.leechers} leech)', style: const TextStyle(color: ObsidianColors.outline)),
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
                                backgroundColor: ObsidianColors.surfaceContainerHighest,
                                valueColor: AlwaysStoppedAnimation(
                                  item.state == QueueState.stalled
                                      ? ObsidianColors.error
                                      : (item.state == QueueState.finalizing ? ObsidianColors.tertiary : ObsidianColors.primary),
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
                                    style: ObsidianTypography.monoCode(color: ObsidianColors.primary, fontSize: 10),
                                    overflow: TextOverflow.ellipsis,
                                  ),
                                ),
                                const SizedBox(width: 4),
                                Flexible(
                                  child: Text(
                                    item.formattedSize,
                                    style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 9),
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
                            color: item.state == QueueState.stalled ? ObsidianColors.error : ObsidianColors.primary,
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
                                child: Icon(Icons.pause_rounded, size: 16, color: ObsidianColors.outline),
                              ),
                            ),
                            InkWell(
                              onTap: () {},
                              child: const Padding(
                                padding: EdgeInsets.all(4.0),
                                child: Icon(Icons.delete_outline_rounded, size: 16, color: ObsidianColors.outline),
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
                      style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10),
                      children: [
                        const TextSpan(text: 'Selected: '),
                        TextSpan(text: activeItem?.title ?? 'None', style: const TextStyle(color: ObsidianColors.primary, fontWeight: FontWeight.bold)),
                        const TextSpan(text: ' | qBittorrent Slot Allocation: 4/12'),
                      ],
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                Text('Sort: Priority · Descending', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10)),
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
      decoration: BoxDecoration(color: bg, borderRadius: BorderRadius.circular(12)),
      child: Text(label, style: ObsidianTypography.monoCode(color: fg, fontSize: 10, fontWeight: FontWeight.w500)),
    );
  }

  Widget _buildInspectorDrawer(QueueItem item) {
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
                child: const Icon(Icons.movie_rounded, size: 24, color: ObsidianColors.outline),
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
                      style: ObsidianTypography.monoCode(color: ObsidianColors.primary, fontSize: 10),
                    ),
                    Text(
                      'Hash: 4a8f9c2e0b11...d81e05a',
                      style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 9),
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
              _buildSmallActionPill(Icons.replay_rounded, 'Force Recheck', ObsidianColors.tertiary),
              _buildSmallActionPill(Icons.folder_open_rounded, 'Open Folder', ObsidianColors.outline, onTap: () => _openFileExplorer('/mnt/anime/TV')),
              _buildSmallActionPill(Icons.link_rounded, 'Magnet', ObsidianColors.outline),
              _buildSmallActionPill(Icons.cancel_rounded, 'Abort', ObsidianColors.error),
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
                  style: ObsidianTypography.labelBadge(color: ObsidianColors.outline, fontSize: 10),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 8),
              Text('qBit Slot #2', style: ObsidianTypography.monoCode(color: ObsidianColors.primary, fontSize: 10)),
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
                _buildMetricRow('Total Size', '${item.formattedSize} (${(item.progress * 100).toInt()}% done)'),
                _buildMetricRow('Peers Swarm', '${item.seeders} seeds · ${item.leechers} leech'),
                _buildMetricRow('Share Ratio', '0.14 (Uploading ${item.uploadSpeed})'),
                _buildMetricRow('ETA Countdown', '~${item.eta} remaining', valueColor: ObsidianColors.primary),
                const SizedBox(height: 6),
                Text('Destination Hardlink Target', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline, fontSize: 9)),
                const SizedBox(height: 3),
                Container(
                  padding: const EdgeInsets.all(6),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLowest,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    '/mnt/anime/TV/${item.title}.mkv',
                    style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 9),
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
                  style: ObsidianTypography.labelBadge(color: ObsidianColors.outline, fontSize: 10),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 8),
              Text('Prowlarr Parser v3', style: ObsidianTypography.monoCode(color: ObsidianColors.tertiary, fontSize: 10)),
            ],
          ),
          const SizedBox(height: 8),

          // Candidate 1 (Chosen)
          Container(
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.primaryContainer.withOpacity(0.15),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: ObsidianColors.primary.withOpacity(0.3)),
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
                            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1.5),
                            decoration: BoxDecoration(color: ObsidianColors.primary, borderRadius: BorderRadius.circular(3)),
                            child: Text('CHOSEN', style: ObsidianTypography.labelBadge(color: ObsidianColors.onPrimary, fontSize: 9)),
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
                    Text('98 / 100', style: ObsidianTypography.monoMetric(color: ObsidianColors.primary, fontWeight: FontWeight.bold, fontSize: 12)),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  'Tier-1 group • Native 1080p rip • Opus Japanese 2.0 • Fast seed availability.',
                  style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant, fontSize: 10),
                ),
                const SizedBox(height: 4),
                Wrap(
                  spacing: 8,
                  children: [
                    Text('+40 Group Tier', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 9)),
                    Text('+30 Audio Match', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 9)),
                    Text('+28 Seeder Density', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 9)),
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
                            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                            decoration: BoxDecoration(color: ObsidianColors.surfaceContainerHighest, borderRadius: BorderRadius.circular(3)),
                            child: Text('REJECTED', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline, fontSize: 8)),
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
                      Text('Equal quality, but initial swarm count 48 vs 142.', style: ObsidianTypography.bodySm(color: ObsidianColors.outline, fontSize: 9)),
                    ],
                  ),
                ),
                const SizedBox(width: 6),
                Text('92 / 100', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10)),
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
              style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10),
              overflow: TextOverflow.ellipsis,
            ),
          ),
          const SizedBox(width: 8),
          Flexible(
            child: Text(
              value,
              style: ObsidianTypography.monoMetric(color: valueColor ?? ObsidianColors.onSurface, fontSize: 10),
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.right,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildSmallActionPill(IconData icon, String label, Color iconColor, {VoidCallback? onTap}) {
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
            Text(label, style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface, fontSize: 10)),
          ],
        ),
      ),
    );
  }

  Widget _buildPill(String label, String value, Color color) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer.withOpacity(0.7),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(label, style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10)),
          Text(value, style: ObsidianTypography.monoMetric(color: color, fontWeight: FontWeight.bold, fontSize: 10)),
        ],
      ),
    );
  }
}
