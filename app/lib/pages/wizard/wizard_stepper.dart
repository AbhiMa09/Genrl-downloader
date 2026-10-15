import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class WizardStepper extends StatelessWidget {
  final int currentStep;

  const WizardStepper({super.key, required this.currentStep});

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

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
      color: ObsidianColors.surfaceContainerLowest,
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: [
            _buildStepItem(
                stepNum: 1,
                title: 'MAL Account',
                subtitle: step1Done ? 'ren_kurokawa' : 'Configure',
                isDone: step1Done,
                isActive: step1Active),
            _buildStepDivider(isDone: step1Done),
            _buildStepItem(
                stepNum: 2,
                title: 'qBittorrent',
                subtitle: step2Done ? ':8080 OK' : 'Connect',
                isDone: step2Done,
                isActive: step2Active),
            _buildStepDivider(isDone: step2Done),
            _buildStepItem(
                stepNum: 3,
                title: 'Root Path',
                subtitle: step3Done ? '/mnt/anime/TV' : 'Destination',
                isDone: step3Done,
                isActive: step3Active),
            _buildStepDivider(isDone: step3Done),
            _buildStepItem(
                stepNum: 4,
                title: 'Quality / Groups',
                subtitle: step4Done ? '1080p • 5 Groups' : 'Preferences',
                isDone: step4Done,
                isActive: step4Active),
            _buildStepDivider(isDone: step4Done),
            _buildStepItem(
                stepNum: 5,
                title: 'Review & Launch',
                subtitle: 'Ready',
                isDone: step5Done,
                isActive: step5Active),
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
  }) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: 22,
          height: 22,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: isActive
                ? ObsidianColors.primary
                : (isDone
                    ? ObsidianColors.tertiary.withOpacity(0.2)
                    : ObsidianColors.surfaceContainerHigh),
          ),
          child: isDone
              ? const Icon(Icons.check_rounded,
                  size: 14, color: ObsidianColors.tertiary)
              : Center(
                  child: Text(
                    '$stepNum',
                    style: ObsidianTypography.labelBadge(
                      color: isActive
                          ? ObsidianColors.onPrimary
                          : ObsidianColors.outline,
                    ),
                  ),
                ),
        ),
        const SizedBox(width: 6),
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              title,
              style: ObsidianTypography.labelBadge(
                color: isActive
                    ? ObsidianColors.primary
                    : ObsidianColors.onSurface,
              ).copyWith(
                  fontWeight: isActive ? FontWeight.bold : FontWeight.normal),
            ),
            Text(
              subtitle,
              style: ObsidianTypography.labelBadge(
                color: isDone ? ObsidianColors.tertiary : ObsidianColors.outline,
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildStepDivider({required bool isDone}) {
    return Container(
      width: 32,
      height: 1.5,
      margin: const EdgeInsets.symmetric(horizontal: 8),
      color: isDone
          ? ObsidianColors.tertiary.withOpacity(0.4)
          : ObsidianColors.primary.withOpacity(0.4),
    );
  }
}
