import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class WizardDailyCapWarning extends StatelessWidget {
  final bool bypassDailyCap;
  final ValueChanged<bool> onToggleBypass;

  const WizardDailyCapWarning({
    super.key,
    required this.bypassDailyCap,
    required this.onToggleBypass,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Icon(Icons.warning_rounded,
                  size: 22, color: ObsidianColors.primary),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Text('Daily Ingestion Cap Exceeded Warning',
                            style: ObsidianTypography.titleSm()
                                .copyWith(fontWeight: FontWeight.bold)),
                        Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 6, vertical: 2),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainer,
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Text('Cap: 30 eps/day',
                              style: ObsidianTypography.monoCode(
                                  color: ObsidianColors.primary)),
                        ),
                      ],
                    ),
                    const SizedBox(height: 4),
                    Text(
                      'Your download cap is configured to 30 episodes/day in client preferences. Committing all 214 backlog episodes will schedule the first 30 today and throttle the remaining 184 into staged daily queues to prevent tracker rate-limits and ISP bandwidth alarms.',
                      style: ObsidianTypography.bodySm(
                          color: ObsidianColors.onSurfaceVariant),
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
                          'Bypass daily cap for initial first-run sync (Queue all 214 immediately without daily delay)',
                          style: ObsidianTypography.bodySm()
                              .copyWith(fontWeight: FontWeight.bold),
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
                          : ObsidianColors.outline),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
