import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/inbox_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/state_views.dart';

class InboxPage extends StatelessWidget {
  const InboxPage({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Header description
          Row(
            children: [
              Text(
                'Action Required (${state.inboxCount})',
                style: ObsidianTypography.headlineLg(),
              ),
              const Spacer(),
              Text(
                'Questions pause only their episode, never the queue.',
                style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
              ),
            ],
          ),
          const SizedBox(height: 20),

          if (state.inboxError != null)
            ErrorStateView(
              title: 'Inbox Query Error',
              message: state.inboxError!,
              onRetry: () => state.refreshInbox(),
              retryLabel: 'Retry Inbox',
            )
          else if (state.isLoadingInbox && state.inbox.isEmpty)
            const LoadingStateView(message: 'Querying pending questions from core engine...')
          else if (state.inbox.isEmpty)
            const EmptyStateView(
              icon: Icons.check_circle_outline_rounded,
              title: 'All Caught Up!',
              description: 'No pending questions in your Inbox. Automation proceeds autonomously without blocking.',
            )
          else
            ...state.inbox.map((question) {
              return _InboxQuestionCard(
                item: question,
                onAnswer: (choiceKey) {
                  state.answerQuestion(question.id, choiceKey);
                },
              );
            }),
        ],
      ),
    );
  }
}

class _InboxQuestionCard extends StatelessWidget {
  final InboxItem item;
  final ValueChanged<String> onAnswer;

  const _InboxQuestionCard({
    required this.item,
    required this.onAnswer,
  });

  @override
  Widget build(BuildContext context) {
    Color badgeColor = ObsidianColors.primary;
    String badgeLabel = 'Question';

    switch (item.type) {
      case QuestionType.fileExists:
        badgeColor = ObsidianColors.warning;
        badgeLabel = 'File Exists Collision';
        break;
      case QuestionType.whichCandidate:
        badgeColor = ObsidianColors.tertiary;
        badgeLabel = 'Tied Release Candidates';
        break;
      case QuestionType.thinSeeds:
        badgeColor = ObsidianColors.error;
        badgeLabel = 'Thin Seed Health';
        break;
      default:
        badgeColor = ObsidianColors.primary;
        badgeLabel = 'Review';
    }

    return Container(
      margin: const EdgeInsets.only(bottom: 16),
      padding: const EdgeInsets.all(18),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border, width: 1),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header Row
          Row(
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2.5),
                decoration: BoxDecoration(
                  color: badgeColor.withOpacity(0.18),
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(color: badgeColor.withOpacity(0.3), width: 1),
                ),
                child: Text(
                  badgeLabel,
                  style: ObsidianTypography.labelBadge(color: badgeColor),
                ),
              ),
              const SizedBox(width: 10),
              Text(
                item.showTitle,
                style: ObsidianTypography.titleSm(),
              ),
              const Spacer(),
              Text(
                'Asked recently',
                style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
              ),
            ],
          ),
          const SizedBox(height: 10),

          // Question Title & Detail
          Text(
            item.title,
            style: ObsidianTypography.headlineMd().copyWith(fontSize: 17),
          ),
          const SizedBox(height: 6),
          Text(
            item.details,
            style: ObsidianTypography.bodyMd(
                color: ObsidianColors.onSurfaceVariant),
          ),
          const SizedBox(height: 16),

          // If candidates comparison
          if (item.candidateA != null && item.candidateB != null) ...[
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLowest,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Candidate 1: ${item.candidateA}',
                      style: ObsidianTypography.monoCode(
                          color: ObsidianColors.primary)),
                  const SizedBox(height: 4),
                  Text('Candidate 2: ${item.candidateB}',
                      style: ObsidianTypography.monoCode(
                          color: ObsidianColors.tertiary)),
                ],
              ),
            ),
            const SizedBox(height: 16),
          ],

          // Action choices buttons
          Wrap(
            spacing: 10,
            runSpacing: 8,
            children: item.choices.map((choice) {
              return ElevatedButton(
                onPressed: () => onAnswer(choice.key),
                style: ElevatedButton.styleFrom(
                  backgroundColor: choice.isDestructive
                      ? ObsidianColors.errorContainer
                      : ObsidianColors.surfaceContainerHighest,
                  foregroundColor: choice.isDestructive
                      ? ObsidianColors.onErrorContainer
                      : ObsidianColors.onSurface,
                  elevation: 0,
                  padding: const EdgeInsets.symmetric(
                      horizontal: 14, vertical: 10),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                    side: BorderSide(
                      color: choice.isDestructive
                          ? ObsidianColors.error.withOpacity(0.4)
                          : ObsidianColors.border,
                      width: 1,
                    ),
                  ),
                ),
                child: Text(
                  choice.label,
                  style: ObsidianTypography.titleSm(
                    color: choice.isDestructive
                        ? ObsidianColors.error
                        : ObsidianColors.onSurface,
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
