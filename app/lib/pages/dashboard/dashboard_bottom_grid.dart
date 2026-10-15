import 'package:flutter/material.dart';
import '../../models/activity_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class DashboardBottomGrid extends StatelessWidget {
  final StateProvider state;

  const DashboardBottomGrid({
    super.key,
    required this.state,
  });

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final isStacked = constraints.maxWidth < 900;
        return Flex(
          direction: isStacked ? Axis.vertical : Axis.horizontal,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Left 2 Cols: Recent Library Ingestions & Re-links
            Expanded(
              flex: isStacked ? 0 : 2,
              child: Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLow,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Wrap(
                      alignment: WrapAlignment.spaceBetween,
                      crossAxisAlignment: WrapCrossAlignment.center,
                      spacing: 8,
                      runSpacing: 4,
                      children: [
                        Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            const Icon(Icons.assignment_turned_in_rounded, size: 18, color: ObsidianColors.primary),
                            const SizedBox(width: 8),
                            Text('Recent Library Ingestions & Re-links', style: ObsidianTypography.headlineMd(fontSize: 16)),
                          ],
                        ),
                        Text('Hardlink Mode: ON', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 11)),
                      ],
                    ),
                    const SizedBox(height: 12),
                    if (state.events.isNotEmpty)
                      ...state.events.take(3).map((act) => Padding(
                            padding: const EdgeInsets.only(bottom: 8.0),
                            child: _buildFeedRow(
                              dotColor: act.level == ActivityLevel.error
                                  ? ObsidianColors.error
                                  : (act.level == ActivityLevel.warn ? ObsidianColors.warning : ObsidianColors.primary),
                              title: act.message,
                              group: act.jobId != null ? 'Job #${act.jobId}' : (act.malId != null ? 'Show #${act.malId}' : 'Core'),
                              target: 'Event #${act.id} recorded in events store',
                              time: act.formattedTime,
                            ),
                          ))
                    else
                      Padding(
                        padding: const EdgeInsets.symmetric(vertical: 16.0),
                        child: Center(
                          child: Text(
                            'No recent ingestions or disk links yet',
                            style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                          ),
                        ),
                      ),
                  ],
                ),
              ),
            ),
            if (!isStacked) const SizedBox(width: 14) else const SizedBox(height: 14),

            // Right 1 Col: Tracker / RSS Matrix
            Expanded(
              flex: isStacked ? 0 : 1,
              child: Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLow,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Expanded(
                          child: Row(
                            children: [
                              const Icon(Icons.sensors_rounded, size: 18, color: ObsidianColors.tertiary),
                              const SizedBox(width: 8),
                              Expanded(
                                child: Text('Tracker / RSS Matrix', style: ObsidianTypography.headlineMd(fontSize: 16), overflow: TextOverflow.ellipsis),
                              ),
                            ],
                          ),
                        ),
                        IconButton(
                          icon: const Icon(Icons.refresh_rounded, size: 16, color: ObsidianColors.outline),
                          onPressed: () {},
                          padding: EdgeInsets.zero,
                          constraints: const BoxConstraints(),
                        ),
                      ],
                    ),
                    const SizedBox(height: 12),
                    _buildTrackerRow('Nyaa.si (Gate Limiter)', 'Gate: 2 req/s', state.isCoreConnected ? 'ACTIVE' : 'OFFLINE', ObsidianColors.tertiary),
                    const SizedBox(height: 6),
                    _buildTrackerRow('MyAnimeList API', state.isMalConnected ? 'OAuth Linked' : 'Public Profile', state.isMalConnected ? 'AUTH OK' : 'READY', ObsidianColors.primary),
                    const SizedBox(height: 6),
                    _buildTrackerRow('Jikan Moe API', 'Rate: 50/min', 'METADATA OK', ObsidianColors.tertiary),
                    const SizedBox(height: 12),
                    Container(
                      padding: const EdgeInsets.all(8),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainer.withOpacity(0.6),
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text('Poll Interval: 15 mins', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 10)),
                                Text('Sync Queue: 0 pending', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10)),
                              ],
                            ),
                          ),
                          Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              const Icon(Icons.shield_rounded, size: 16, color: ObsidianColors.primary),
                              const SizedBox(width: 4),
                              Text('100% SLA', style: ObsidianTypography.monoMetric(color: ObsidianColors.primary, fontSize: 11, fontWeight: FontWeight.bold)),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        );
      },
    );
  }

  Widget _buildFeedRow({
    required Color dotColor,
    required String title,
    required String group,
    required String target,
    required String time,
  }) {
    return Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer.withOpacity(0.4),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        children: [
          Container(
            width: 6,
            height: 6,
            decoration: BoxDecoration(color: dotColor, shape: BoxShape.circle),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        title,
                        style: ObsidianTypography.titleSm(fontSize: 12),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    const SizedBox(width: 6),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHighest,
                        borderRadius: BorderRadius.circular(3),
                      ),
                      child: Text(group, style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary, fontSize: 9)),
                    ),
                  ],
                ),
                Text(
                  target,
                  style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          const SizedBox(width: 8),
          Text(time, style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 10)),
        ],
      ),
    );
  }

  Widget _buildTrackerRow(String name, String ping, String statusBadge, Color dotColor) {
    return Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Expanded(
            child: Row(
              children: [
                Container(width: 6, height: 6, decoration: BoxDecoration(color: dotColor, shape: BoxShape.circle)),
                const SizedBox(width: 8),
                Expanded(child: Text(name, style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface, fontSize: 11), overflow: TextOverflow.ellipsis)),
              ],
            ),
          ),
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(ping, style: ObsidianTypography.monoMetric(color: ObsidianColors.tertiary, fontSize: 11)),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1.5),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(statusBadge, style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurfaceVariant, fontSize: 9)),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
