import 'package:flutter/material.dart';

import '../../theme/colors.dart';
import '../../theme/typography.dart';

class WizardDailyCapWarning extends StatelessWidget {
  final bool bypassDailyCap;
  final ValueChanged<bool> onToggleBypass;
  final int dailyCap;
  final int plannedEpisodes;

  const WizardDailyCapWarning({
    super.key,
    required this.bypassDailyCap,
    required this.onToggleBypass,
    this.dailyCap = 25,
    this.plannedEpisodes = 11,
  });

  @override
  Widget build(BuildContext context) {
    final isExceeded = plannedEpisodes > dailyCap;

    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: isExceeded
            ? ObsidianColors.surfaceContainerHigh
            : ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(
                isExceeded ? Icons.warning_rounded : Icons.info_outline_rounded,
                size: 22,
                color: isExceeded
                    ? ObsidianColors.primary
                    : ObsidianColors.tertiary,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Text(
                          isExceeded
                              ? 'Daily Ingestion Cap Exceeded Warning'
                              : 'Daily Ingestion Budget Info',
                          style: ObsidianTypography.titleSm().copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                        Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 6,
                            vertical: 2,
                          ),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainer,
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Text(
                            'First-sync cap: $dailyCap (daily cap: 30)',
                            style: ObsidianTypography.monoCode(
                              color: ObsidianColors.primary,
                            ),
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 4),
                    Text(
                      plannedEpisodes > dailyCap
                          ? '$plannedEpisodes planned, exceeding the $dailyCap first-sync cap. The first $dailyCap episodes will be queued today; remaining ${plannedEpisodes - dailyCap} staged for tomorrow.'
                          : '$plannedEpisodes planned, within the $dailyCap first-sync cap. All $plannedEpisodes backlog episodes will be queued immediately without throttle.',
                      style: ObsidianTypography.bodySm(
                        color: ObsidianColors.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 10),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest.withOpacity(0.6),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Expanded(
                  child: Row(
                    children: [
                      Checkbox(
                        value: bypassDailyCap,
                        onChanged: (v) => onToggleBypass(v ?? false),
                        activeColor: ObsidianColors.primary,
                      ),
                      Flexible(
                        child: Text(
                          'Bypass daily cap for initial first-run sync (Queue all $plannedEpisodes immediately without daily delay)',
                          style: ObsidianTypography.bodySm().copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                Text(
                  bypassDailyCap
                      ? 'Full queue release'
                      : 'Staged batching active',
                  style: ObsidianTypography.labelBadge(
                    color: bypassDailyCap
                        ? ObsidianColors.warning
                        : ObsidianColors.outline,
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
