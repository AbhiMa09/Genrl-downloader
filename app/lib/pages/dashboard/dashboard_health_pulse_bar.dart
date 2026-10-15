import 'package:flutter/material.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class DashboardHealthPulseBar extends StatelessWidget {
  final StateProvider state;

  const DashboardHealthPulseBar({
    super.key,
    required this.state,
  });

  @override
  Widget build(BuildContext context) {
    final activeTorrents = state.status.activeTorrents > 0
        ? state.status.activeTorrents
        : state.activeQueueCount;
    final monitoredShows = state.status.totalShows > 0
        ? state.status.totalShows
        : state.shows.length;
    final nextShow = state.shows.isNotEmpty ? state.shows.first.title : '-';

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Expanded(
            child: Wrap(
              crossAxisAlignment: WrapCrossAlignment.center,
              spacing: 12,
              runSpacing: 4,
              children: [
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(Icons.download_rounded, size: 14, color: ObsidianColors.tertiary),
                    const SizedBox(width: 4),
                    Text(
                      '$activeTorrents Active Torrents',
                      style: ObsidianTypography.monoMetric(color: ObsidianColors.onSurface, fontSize: 12),
                    ),
                  ],
                ),
                Text('/', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(Icons.bookmark_added_rounded, size: 14, color: ObsidianColors.primary),
                    const SizedBox(width: 4),
                    Text(
                      '$monitoredShows Monitored Shows',
                      style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 12),
                    ),
                  ],
                ),
                Text('/', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(Icons.update_rounded, size: 14, color: ObsidianColors.secondary),
                    const SizedBox(width: 4),
                    RichText(
                      text: TextSpan(
                        style: ObsidianTypography.monoCode(fontSize: 12),
                        children: [
                          const TextSpan(text: 'Next Release: ', style: TextStyle(color: ObsidianColors.primaryFixedDim)),
                          TextSpan(text: nextShow, style: const TextStyle(color: ObsidianColors.onSurface, fontWeight: FontWeight.bold)),
                          TextSpan(
                            text: state.shows.isNotEmpty && state.shows.first.airingStatus.isNotEmpty
                                ? ' (${state.shows.first.airingStatus})'
                                : '',
                            style: const TextStyle(color: ObsidianColors.outline),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('CRON: ACTIVE', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline, fontSize: 10)),
              ),
              const SizedBox(width: 6),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('PARSE: NYAA.SI', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary, fontSize: 10)),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
