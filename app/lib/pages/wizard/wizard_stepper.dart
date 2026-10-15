import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class WizardStepper extends StatelessWidget {
  final int currentStep;

  const WizardStepper({super.key, required this.currentStep});

  @override
  Widget build(BuildContext context) {
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
                subtitle: 'ren_kurokawa',
                isDone: true,
                isActive: false),
            _buildStepDivider(isDone: true),
            _buildStepItem(
                stepNum: 2,
                title: 'qBittorrent',
                subtitle: ':8080 OK',
                isDone: true,
                isActive: false),
            _buildStepDivider(isDone: true),
            _buildStepItem(
                stepNum: 3,
                title: 'Root Path',
                subtitle: '/mnt/anime/TV',
                isDone: true,
                isActive: false),
            _buildStepDivider(isDone: true),
            _buildStepItem(
                stepNum: 4,
                title: 'Quality / Groups',
                subtitle: '1080p • 5 Groups',
                isDone: true,
                isActive: false),
            _buildStepDivider(isDone: false),
            _buildStepItem(
                stepNum: 5,
                title: 'Review & Launch',
                subtitle: 'Ready',
                isDone: false,
                isActive: true),
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
