import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/inbox_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/state_views.dart';

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

    final dynamicItems = state.inbox.where((i) => !_resolvedCards.contains('item-${i.id}')).toList();

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
          _buildHeader(context, state, pendingCount),
          const SizedBox(height: 16),

          // 2. Filter Category Tabs Strip
          _buildCategoryTabs(pendingCount),
          const SizedBox(height: 20),

          // 3. Dynamic Decision Stack Cards
          ...dynamicItems.map((item) => Padding(
            padding: const EdgeInsets.only(bottom: 20),
            child: _buildDynamicCard(item, state),
          )),
        ],
      ),
    );
  }

  Widget _buildHeader(BuildContext context, StateProvider state, int pendingCount) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Wrap(
          alignment: WrapAlignment.spaceBetween,
          crossAxisAlignment: WrapCrossAlignment.center,
          spacing: 16,
          runSpacing: 10,
          children: [
            Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text('Inbox', style: ObsidianTypography.headlineLg()),
                    const SizedBox(width: 10),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                      decoration: BoxDecoration(
                        color: ObsidianColors.primaryContainer.withOpacity(0.2),
                        borderRadius: BorderRadius.circular(12),
                      ),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Container(
                            width: 6,
                            height: 6,
                            decoration: const BoxDecoration(color: ObsidianColors.primary, shape: BoxShape.circle),
                          ),
                          const SizedBox(width: 6),
                          Text('$pendingCount Decisions Pending', style: ObsidianTypography.monoMetric(color: ObsidianColors.primary, fontWeight: FontWeight.bold, fontSize: 11)),
                        ],
                      ),
                    ),
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHigh,
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text('parser-queue: halted', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10)),
                    ),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  'Automated ingestion pipeline pauses requiring user intervention or rule clarification before commit.',
                  style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                ),
              ],
            ),

            // Search & Batch Actions
            Wrap(
              spacing: 8,
              runSpacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                Container(
                  width: 220,
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
                            hintText: 'Filter decisions...',
                            hintStyle: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                            border: InputBorder.none,
                            isDense: true,
                          ),
                          onChanged: (_) => setState(() {}),
                        ),
                      ),
                    ],
                  ),
                ),

                OutlinedButton.icon(
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('Notifications suppressed for 60 minutes')),
                    );
                  },
                  icon: const Icon(Icons.snooze_rounded, size: 14, color: ObsidianColors.outline),
                  label: Text('Ignore 1h', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 11)),
                  style: OutlinedButton.styleFrom(
                    backgroundColor: ObsidianColors.surfaceContainerHigh,
                    side: const BorderSide(color: ObsidianColors.border),
                    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                  ),
                ),

                ElevatedButton.icon(
                  onPressed: () {
                    setState(() {
                      _resolvedCards.addAll(['card-1', 'card-2', 'card-3', 'card-4', 'card-5']);
                    });
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('All decisions batch resolved with top-ranked algorithms!'), backgroundColor: ObsidianColors.success),
                    );
                  },
                  icon: const Icon(Icons.done_all_rounded, size: 16),
                  label: Text('Batch Resolve All', style: ObsidianTypography.titleSm(color: ObsidianColors.onPrimary, fontSize: 12)),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: ObsidianColors.primary,
                    foregroundColor: ObsidianColors.onPrimary,
                    elevation: 0,
                    padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                  ),
                ),
              ],
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildCategoryTabs(int pendingCount) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: Row(
            children: [
              _buildPillTab('all', 'Action Required ($pendingCount)'),
              const SizedBox(width: 6),
              _buildPillTab('matches', 'Matches (0)'),
              const SizedBox(width: 6),
              _buildPillTab('seeds', 'Seeds (0)'),
              const SizedBox(width: 6),
              _buildPillTab('files', 'Files ($pendingCount)'),
              const SizedBox(width: 6),
              _buildPillTab('batches', 'Batches (0)'),
            ],
          ),
        ),
        Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text('Sort: ', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 11)),
            Text('Priority & Age', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface, fontWeight: FontWeight.bold, fontSize: 11)),
          ],
        ),
      ],
    );
  }

  Widget _buildPillTab(String key, String label) {
    final isSelected = _selectedCategory == key;
    return InkWell(
      onTap: () => setState(() => _selectedCategory = key),
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: isSelected ? ObsidianColors.surfaceContainerHighest : Colors.transparent,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Text(
          label,
          style: ObsidianTypography.bodySm(
            color: isSelected ? ObsidianColors.primary : ObsidianColors.outline,
            fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
            fontSize: 11,
          ),
        ),
      ),
    );
  }

  Widget _buildDynamicCard(InboxItem item, StateProvider state) {
    Color accentColor = ObsidianColors.tertiary;
    String categoryTag = 'CONFLICT DETECTED';
    IconData tagIcon = Icons.difference_rounded;
    Color tagColor = ObsidianColors.tertiary;

    if (item.type == QuestionType.thinSeeds) {
      accentColor = ObsidianColors.secondary;
      categoryTag = 'PEER HEALTH WARNING';
      tagIcon = Icons.network_check_rounded;
      tagColor = ObsidianColors.secondary;
    } else if (item.type == QuestionType.matchReview) {
      accentColor = ObsidianColors.primary;
      categoryTag = 'MATCH REVIEW';
      tagIcon = Icons.verified_user_rounded;
      tagColor = ObsidianColors.primary;
    } else if (item.type == QuestionType.batchOnly) {
      accentColor = ObsidianColors.primary;
      categoryTag = 'BATCH PACK DISCOVERED';
      tagIcon = Icons.folder_zip_rounded;
      tagColor = ObsidianColors.primary;
    }

    return _DecisionCardFrame(
      accentColor: accentColor,
      categoryTag: categoryTag,
      tagIcon: tagIcon,
      tagColor: tagColor,
      metaText: '${item.showTitle} · Priority Job #${item.jobId ?? item.id}',
      title: item.title,
      description: item.details,
      autoRuleText: 'Always apply this resolution rule for ${item.showTitle}',
      ruleId: '#auto-resolve-${item.id}',
      onSecondaryAction: () => _resolveCard('item-${item.id}', 'Dismissed ${item.title}'),
      secondaryActionLabel: 'Dismiss',
      content: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Text(
              item.details,
              style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant, fontSize: 11),
            ),
          ),
          const SizedBox(height: 14),
          Wrap(
            spacing: 10,
            runSpacing: 10,
            children: item.choices.map((choice) {
              final isPrimary = choice.key == item.choices.first.key;
              return ElevatedButton(
                onPressed: () {
                  _resolveCard('item-${item.id}', '${choice.label} for ${item.title}');
                  state.answerInboxQuestion(item.id, choice.key);
                },
                style: ElevatedButton.styleFrom(
                  backgroundColor: isPrimary ? ObsidianColors.primary : ObsidianColors.surfaceContainerHigh,
                  foregroundColor: isPrimary ? ObsidianColors.onPrimary : ObsidianColors.onSurface,
                  side: isPrimary ? BorderSide.none : const BorderSide(color: ObsidianColors.border),
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
                child: Text(
                  choice.label,
                  style: TextStyle(
                    color: isPrimary ? ObsidianColors.onPrimary : ObsidianColors.onSurface,
                    fontWeight: FontWeight.w600,
                    fontSize: 12,
                  ),
                ),
              );
            }).toList(),
          ),
        ],
      ),
    );
  }

}

class _DecisionCardFrame extends StatelessWidget {
  final Color accentColor;
  final String categoryTag;
  final IconData tagIcon;
  final Color tagColor;
  final String metaText;
  final String title;
  final String description;
  final Widget content;
  final String autoRuleText;
  final String ruleId;
  final VoidCallback? onSecondaryAction;
  final String? secondaryActionLabel;

  const _DecisionCardFrame({
    required this.accentColor,
    required this.categoryTag,
    required this.tagIcon,
    required this.tagColor,
    required this.metaText,
    required this.title,
    required this.description,
    required this.content,
    required this.autoRuleText,
    required this.ruleId,
    this.onSecondaryAction,
    this.secondaryActionLabel,
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
      child: Stack(
        children: [
          Positioned(
            left: 0,
            top: 0,
            bottom: 0,
            width: 4,
            child: Container(color: accentColor),
          ),
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
            // Header Row
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                            decoration: BoxDecoration(color: tagColor.withOpacity(0.2), borderRadius: BorderRadius.circular(4)),
                            child: Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                Icon(tagIcon, size: 12, color: tagColor),
                                const SizedBox(width: 4),
                                Text(categoryTag, style: ObsidianTypography.labelBadge(color: tagColor, fontSize: 9)),
                              ],
                            ),
                          ),
                          const SizedBox(width: 8),
                          Text(metaText, style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10)),
                        ],
                      ),
                      const SizedBox(height: 6),
                      Text(title, style: ObsidianTypography.headlineMd(fontSize: 16)),
                      const SizedBox(height: 2),
                      Text(description, style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant, fontSize: 11)),
                    ],
                  ),
                ),
                if (secondaryActionLabel != null)
                  OutlinedButton(
                    onPressed: onSecondaryAction,
                    style: OutlinedButton.styleFrom(
                      side: const BorderSide(color: ObsidianColors.border),
                      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                    ),
                    child: Text(secondaryActionLabel!, style: ObsidianTypography.monoCode(color: ObsidianColors.error, fontSize: 10)),
                  ),
              ],
            ),
            const SizedBox(height: 14),

            // Custom Body Content
            content,
            const SizedBox(height: 14),

            // Footer with "Always do this" auto-rule checkbox
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLowest.withOpacity(0.5),
                borderRadius: BorderRadius.circular(6),
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Row(
                    children: [
                      const Icon(Icons.check_box_outline_blank_rounded, size: 16, color: ObsidianColors.primary),
                      const SizedBox(width: 6),
                      Text(autoRuleText, style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant, fontSize: 10)),
                    ],
                  ),
                  Text('Rule ID: $ruleId', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 9)),
                ],
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
