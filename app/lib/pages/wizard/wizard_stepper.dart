import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../widgets/loading/ingestion_progress.dart';

class WizardStepper extends StatelessWidget {
  final int currentStep;
  final StateProvider? state;

  const WizardStepper({super.key, required this.currentStep, this.state});

  int _stageNumber(IngestionStage? stage) {
    switch (stage) {
      case IngestionStage.malSync:
        return 1;
      case IngestionStage.metadata:
        return 2;
      case IngestionStage.planQueue:
        return 3;
      case IngestionStage.createJobs:
        return 4;
      default:
        return 1;
    }
  }

  @override
  Widget build(BuildContext context) {
    final step1Done = currentStep > 0;
    final step1Active = currentStep == 0;
    final step2Done = currentStep > 1;
    final step2Active = currentStep == 1;
    final step3Done = currentStep > 2;
    final step3Active = currentStep == 2;
    final step4Done = currentStep > 3;
    final step4Active = currentStep == 3;
    final step5Done = currentStep > 4;
    final step5Active = currentStep == 4;

    final malUser = state?.malUsername != null && state!.malUsername!.isNotEmpty
        ? state!.malUsername!
        : (step1Done ? 'akumasan2' : 'Configure');
    final rootPath =
        state?.libraryFolder != null && state!.libraryFolder.isNotEmpty
        ? state!.libraryFolder
        : (step3Done ? 'Configured' : 'Destination');
    final qbitOk = state?.status.qbitConnected ?? false;
    final qbitSub = step2Done
        ? (qbitOk ? ':8080 OK' : 'Unreachable')
        : 'Connect';

    final isFailed = state?.ingestionStage == IngestionStage.failed;
    final isBlocked = !qbitOk && currentStep == 4;
    final lastSubtitle = isFailed
        ? 'Failed at Stage ${_stageNumber(state?.failedStage)}'
        : (isBlocked ? 'Blocked' : (step5Done ? 'Finished' : 'Ready'));

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
      color: ObsidianColors.surfaceContainerLowest,
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: [
            _buildStepItem(
              stepNum: 1,
              title: 'MAL Account',
              subtitle: malUser,
              isDone: step1Done,
              isActive: step1Active,
            ),
            _buildStepDivider(isDone: step1Done),
            _buildStepItem(
              stepNum: 2,
              title: 'qBittorrent',
              subtitle: qbitSub,
              isDone: step2Done,
              isActive: step2Active,
              isError: step2Done && !qbitOk,
            ),
            _buildStepDivider(isDone: step2Done),
            _buildStepItem(
              stepNum: 3,
              title: 'Root Path',
              subtitle: rootPath,
              isDone: step3Done,
              isActive: step3Active,
            ),
            _buildStepDivider(isDone: step3Done),
            _buildStepItem(
              stepNum: 4,
              title: 'Quality / Groups',
              subtitle: step4Done ? '1080p • 5 Groups' : 'Preferences',
              isDone: step4Done,
              isActive: step4Active,
            ),
            _buildStepDivider(isDone: step4Done),
            _buildStepItem(
              stepNum: 5,
              title: 'Review & Launch',
              subtitle: lastSubtitle,
              isDone: step5Done,
              isActive: step5Active,
              isFailed: isFailed,
              isError: isBlocked,
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildStepItem({
    required int stepNum,
    required String title,
    required String subtitle,
    required bool isDone,
    required bool isActive,
    bool isFailed = false,
    bool isError = false,
  }) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: 22,
          height: 22,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: (isFailed || isError)
                ? ObsidianColors.error.withOpacity(0.2)
                : (isActive
                      ? ObsidianColors.primary
                      : (isDone
                            ? ObsidianColors.tertiary.withOpacity(0.2)
                            : ObsidianColors.surfaceContainerHigh)),
          ),
          child: isFailed
              ? const Icon(
                  Icons.close_rounded,
                  size: 14,
                  color: ObsidianColors.error,
                )
              : (isError
                    ? const Icon(
                        Icons.warning_amber_rounded,
                        size: 14,
                        color: ObsidianColors.error,
                      )
                    : (isDone
                          ? const Icon(
                              Icons.check_rounded,
                              size: 14,
                              color: ObsidianColors.tertiary,
                            )
                          : Center(
                              child: Text(
                                '$stepNum',
                                style: ObsidianTypography.labelBadge(
                                  color: isActive
                                      ? ObsidianColors.onPrimary
                                      : ObsidianColors.outline,
                                ),
                              ),
                            ))),
        ),
        const SizedBox(width: 6),
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              title,
              maxLines: 1,
              softWrap: false,
              overflow: TextOverflow.visible,
              style:
                  ObsidianTypography.labelBadge(
                    color: (isFailed || isError)
                        ? ObsidianColors.error
                        : (isActive
                              ? ObsidianColors.primary
                              : ObsidianColors.onSurface),
                  ).copyWith(
                    fontWeight: isActive || isFailed || isError
                        ? FontWeight.bold
                        : FontWeight.normal,
                  ),
            ),
            Text(
              subtitle,
              maxLines: 1,
              softWrap: false,
              overflow: TextOverflow.visible,
              style: ObsidianTypography.labelBadge(
                color: (isFailed || isError)
                    ? ObsidianColors.error
                    : (isDone
                          ? ObsidianColors.tertiary
                          : ObsidianColors.outline),
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildStepDivider({required bool isDone}) {
    return Container(
      width: 16,
      height: 1.5,
      margin: const EdgeInsets.symmetric(horizontal: 4),
      color: isDone
          ? ObsidianColors.tertiary.withOpacity(0.4)
          : ObsidianColors.primary.withOpacity(0.4),
    );
  }
}
