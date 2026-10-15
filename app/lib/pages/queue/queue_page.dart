import 'dart:io';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../models/queue_item.dart';
import '../../services/state_provider.dart';
import '../../widgets/state_views.dart';
import 'queue_filter_bar.dart';
import 'queue_inspector.dart';
import 'queue_table.dart';
import 'queue_toolbar.dart';

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
            description:
                'Active downloads and jobs will appear here live as they are processed.',
          ),
        ),
      );
    }

    final queueItems = state.queue;

    // Filter items
    final query = _searchController.text.trim().toLowerCase();
    final filteredItems = queueItems.where((item) {
      if (_selectedFilter != 'all') {
        if (_selectedFilter == 'downloading' &&
            item.state != QueueState.downloading) {
          return false;
        }
        if (_selectedFilter == 'finalizing' &&
            item.state != QueueState.finalizing &&
            item.state != QueueState.completed) {
          return false;
        }
        if (_selectedFilter == 'blocked' && item.state != QueueState.blocked) {
          return false;
        }
        if (_selectedFilter == 'failed' && item.state != QueueState.stalled) {
          return false;
        }
        if (_selectedFilter == 'searching' && item.state != QueueState.queued) {
          return false;
        }
        if (_selectedFilter == 'needs_input' &&
            item.seeders > 2 &&
            item.state != QueueState.blocked) {
          return false;
        }
      }
      if (query.isNotEmpty) {
        if (!item.title.toLowerCase().contains(query)) return false;
      }
      return true;
    }).toList();

    final activeItem =
        _selectedItem ?? (filteredItems.isNotEmpty ? filteredItems.first : null);

    return SingleChildScrollView(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // 1. Queue Header & Command Toolbar
          QueueToolbar(
            state: state,
            count: queueItems.length,
          ),

          // 2. Filter Tabs & Quick Search Bar
          QueueFilterBar(
            items: queueItems,
            selectedFilter: _selectedFilter,
            onFilterChanged: (filter) =>
                setState(() => _selectedFilter = filter),
            searchController: _searchController,
            onSearchChanged: () => setState(() {}),
          ),

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
                      child: QueueTable(
                        items: filteredItems,
                        activeItem: activeItem,
                        onSelectItem: (item) =>
                            setState(() => _selectedItem = item),
                      ),
                    ),
                    if (!isStacked)
                      const SizedBox(width: 16)
                    else
                      const SizedBox(height: 16),

                    // Right: Job Details & Decision Inspector
                    if (activeItem != null)
                      SizedBox(
                        width: isStacked ? constraints.maxWidth : 380,
                        child: QueueInspector(
                          item: activeItem,
                          onOpenFolder: _openFileExplorer,
                        ),
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
}
