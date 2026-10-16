import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../widgets/state_views.dart';
import 'inbox_category_tabs.dart';
import 'inbox_dynamic_card.dart';
import 'inbox_header.dart';

class InboxPage extends StatefulWidget {
  const InboxPage({super.key});

  @override
  State<InboxPage> createState() => _InboxPageState();
}

class _InboxPageState extends State<InboxPage> {
  String _selectedCategory = 'all';
  final _searchController = TextEditingController();

  // Resolved cards tracker
  final Set<String> _resolvedCards = {};

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  void _resolveCard(String id, String actionSummary) {
    setState(() {
      _resolvedCards.add(id);
    });
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text('Resolved: $actionSummary'),
        backgroundColor: ObsidianColors.success,
        duration: const Duration(seconds: 2),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    if (state.coreError != null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24.0),
          child: ErrorStateView(
            title: 'Inbox Query Error',
            message: state.coreError!,
            onRetry: () => state.refreshAll(),
            retryLabel: 'Retry Inbox',
          ),
        ),
      );
    }

    if (state.inbox.isEmpty) {
      return const Center(
        child: Padding(
          padding: EdgeInsets.all(24.0),
          child: EmptyStateView(
            title: 'All Caught Up!',
            description: 'No pending questions in your Inbox. Automation policies are handling your library seamlessly.',
          ),
        ),
      );
    }

    final dynamicItems = state.inbox
        .where((i) => !_resolvedCards.contains('item-${i.id}'))
        .toList();

    if (dynamicItems.isEmpty) {
      return const Center(
        child: Padding(
          padding: EdgeInsets.all(24.0),
          child: EmptyStateView(
            title: 'All Caught Up!',
            description: 'No pending questions in your Inbox. Automation policies are handling your library seamlessly.',
          ),
        ),
      );
    }

    final pendingCount = dynamicItems.length;

    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // 1. Page Header & Action Bar
          InboxHeader(
            pendingCount: pendingCount,
            searchController: _searchController,
            onSearchChanged: () => setState(() {}),
            onBatchResolve: () {
              setState(() {
                _resolvedCards.addAll([
                  'card-1',
                  'card-2',
                  'card-3',
                  'card-4',
                  'card-5',
                ]);
              });
              ScaffoldMessenger.of(context).showSnackBar(
                const SnackBar(
                  content: Text(
                    'All decisions batch resolved with top-ranked algorithms!',
                  ),
                  backgroundColor: ObsidianColors.success,
                ),
              );
            },
          ),
          const SizedBox(height: 16),

          // 2. Filter Category Tabs Strip
          InboxCategoryTabs(
            pendingCount: pendingCount,
            selectedCategory: _selectedCategory,
            onSelectCategory: (cat) => setState(() => _selectedCategory = cat),
          ),
          const SizedBox(height: 20),

          // 3. Dynamic Decision Stack Cards
          ...dynamicItems.map(
            (item) => Padding(
              padding: const EdgeInsets.only(bottom: 20),
              child: InboxDynamicCard(
                item: item,
                state: state,
                onResolve: _resolveCard,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
