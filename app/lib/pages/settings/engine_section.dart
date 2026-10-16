import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class SettingsEngineSection extends StatelessWidget {
  final StateProvider state;
  final int concurrency;
  final ValueChanged<int> onConcurrencyChanged;
  final int dailyCap;
  final ValueChanged<int> onDailyCapChanged;

  const SettingsEngineSection({
    super.key,
    required this.state,
    required this.concurrency,
    required this.onConcurrencyChanged,
    required this.dailyCap,
    required this.onDailyCapChanged,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Row(
                  children: [
                    Container(
                      padding: const EdgeInsets.all(6),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHigh,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: const Icon(
                        Icons.tune_rounded,
                        size: 20,
                        color: ObsidianColors.primary,
                      ),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'Download Engine & Concurrency',
                            style: ObsidianTypography.titleSm(),
                            overflow: TextOverflow.ellipsis,
                          ),
                          Text(
                            'Manage active task caps, network buffer quotas, and daily automation throttle limits',
                            style: ObsidianTypography.bodySm(
                              color: ObsidianColors.outline,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  'Scheduler Active',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.tertiary,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),

          Row(
            children: [
              // Concurrency Stepper
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(14),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Expanded(
                            child: Text(
                              'Simultaneous Active Downloads',
                              style: ObsidianTypography.titleSm(),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                          const SizedBox(width: 8),
                          Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 6,
                              vertical: 1.5,
                            ),
                            decoration: BoxDecoration(
                              color: ObsidianColors.primary.withOpacity(0.2),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text(
                              'IO Priority',
                              style: ObsidianTypography.labelBadge(
                                color: ObsidianColors.primary,
                              ),
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 4),
                      Text(
                        'Queue throttles additional snatches to preserve bandwidth priority and avoid ISP congestion flags.',
                        style: ObsidianTypography.bodySm(
                          color: ObsidianColors.outline,
                        ),
                      ),
                      const SizedBox(height: 12),
                      Row(
                        children: [
                          IconButton(
                            onPressed: () {
                              if (concurrency > 1) {
                                onConcurrencyChanged(concurrency - 1);
                              }
                            },
                            icon: const Icon(
                              Icons.remove_circle_outline_rounded,
                            ),
                            color: ObsidianColors.outline,
                          ),
                          Container(
                            width: 60,
                            height: 36,
                            decoration: BoxDecoration(
                              color: ObsidianColors.surfaceContainerLowest,
                              borderRadius: BorderRadius.circular(6),
                            ),
                            alignment: Alignment.center,
                            child: Text(
                              '$concurrency',
                              style: ObsidianTypography.headlineMd(
                                color: ObsidianColors.primary,
                              ),
                            ),
                          ),
                          IconButton(
                            onPressed: () {
                              if (concurrency < 16) {
                                onConcurrencyChanged(concurrency + 1);
                              }
                            },
                            icon: const Icon(Icons.add_circle_outline_rounded),
                            color: ObsidianColors.outline,
                          ),
                          const Spacer(),
                          Text(
                            'Default: 2 streams',
                            style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.onSurfaceVariant,
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 12),

              // Daily Cap Stepper
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(14),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Expanded(
                            child: Text(
                              'Daily Download Limit (Episodes)',
                              style: ObsidianTypography.titleSm(),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                          const SizedBox(width: 8),
                          Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 6,
                              vertical: 1.5,
                            ),
                            decoration: BoxDecoration(
                              color: ObsidianColors.tertiary.withOpacity(0.2),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text(
                              'Tracker Safety',
                              style: ObsidianTypography.labelBadge(
                                color: ObsidianColors.tertiary,
                              ),
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 4),
                      Text(
                        'Maximum automated episode downloads allowed within a 24-hour rolling window to preserve account ratios.',
                        style: ObsidianTypography.bodySm(
                          color: ObsidianColors.outline,
                        ),
                      ),
                      const SizedBox(height: 12),
                      Row(
                        children: [
                          IconButton(
                            onPressed: () {
                              if (dailyCap > 5) onDailyCapChanged(dailyCap - 5);
                            },
                            icon: const Icon(
                              Icons.remove_circle_outline_rounded,
                            ),
                            color: ObsidianColors.outline,
                          ),
                          Container(
                            width: 60,
                            height: 36,
                            decoration: BoxDecoration(
                              color: ObsidianColors.surfaceContainerLowest,
                              borderRadius: BorderRadius.circular(6),
                            ),
                            alignment: Alignment.center,
                            child: Text(
                              '$dailyCap',
                              style: ObsidianTypography.headlineMd(
                                color: ObsidianColors.tertiary,
                              ),
                            ),
                          ),
                          IconButton(
                            onPressed: () {
                              if (dailyCap < 200) {
                                onDailyCapChanged(dailyCap + 5);
                              }
                            },
                            icon: const Icon(Icons.add_circle_outline_rounded),
                            color: ObsidianColors.outline,
                          ),
                          const Spacer(),
                          Text(
                            'Default: 30 eps/day',
                            style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.onSurfaceVariant,
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 14),

          // Micro notification banner
          Container(
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainer,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Expanded(
                  child: Row(
                    children: [
                      const Icon(
                        Icons.info_outline_rounded,
                        size: 16,
                        color: ObsidianColors.primary,
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          'All changes take effect immediately on local daemon via UNIX socket: /run/genrl/core.sock',
                          style: ObsidianTypography.monoCode(
                            color: ObsidianColors.outline,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                Text(
                  'Last written: 14:22:09 UTC',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.onSurfaceVariant,
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
