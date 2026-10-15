import 'package:flutter/material.dart';
import '../../models/inbox_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'decision_card_frame.dart';

class InboxDynamicCard extends StatelessWidget {
  final InboxItem item;
  final StateProvider state;
  final void Function(String id, String summary) onResolve;

  const InboxDynamicCard({
    super.key,
    required this.item,
    required this.state,
    required this.onResolve,
  });

  @override
  Widget build(BuildContext context) {
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

    return DecisionCardFrame(
      accentColor: accentColor,
      categoryTag: categoryTag,
      tagIcon: tagIcon,
      tagColor: tagColor,
      metaText: '${item.showTitle} · Priority Job #${item.jobId ?? item.id}',
      title: item.title,
      description: item.details,
      autoRuleText: 'Always apply this resolution rule for ${item.showTitle}',
      ruleId: '#auto-resolve-${item.id}',
      onSecondaryAction: () =>
          onResolve('item-${item.id}', 'Dismissed ${item.title}'),
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
              style: ObsidianTypography.bodySm(
                  color: ObsidianColors.onSurfaceVariant, fontSize: 11),
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
                  onResolve(
                      'item-${item.id}', '${choice.label} for ${item.title}');
                  state.answerInboxQuestion(item.id, choice.key);
                },
                style: ElevatedButton.styleFrom(
                  backgroundColor: isPrimary
                      ? ObsidianColors.primary
                      : ObsidianColors.surfaceContainerHigh,
                  foregroundColor: isPrimary
                      ? ObsidianColors.onPrimary
                      : ObsidianColors.onSurface,
                  side: isPrimary
                      ? BorderSide.none
                      : const BorderSide(color: ObsidianColors.border),
                  padding: const EdgeInsets.symmetric(
                      horizontal: 16, vertical: 10),
                  shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8)),
                ),
                child: Text(
                  choice.label,
                  style: TextStyle(
                    color: isPrimary
                        ? ObsidianColors.onPrimary
                        : ObsidianColors.onSurface,
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
