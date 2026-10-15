import 'package:flutter/material.dart';
import '../../models/activity_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class ActivityTimelineSection extends StatelessWidget {
  final List<ActivityItem> events;
  final StateProvider state;

  const ActivityTimelineSection({
    super.key,
    required this.events,
    required this.state,
  });

  @override
  Widget build(BuildContext context) {
    if (events.isEmpty) {
      return Container(
        padding: const EdgeInsets.all(32),
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainerLow,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: ObsidianColors.border),
        ),
        child: Center(
          child: Column(
            children: [
              const Icon(Icons.notes_rounded,
                  size: 36, color: ObsidianColors.outline),
              const SizedBox(height: 12),
              Text('No events match the selected criteria',
                  style:
                      ObsidianTypography.bodyMd(color: ObsidianColors.outline)),
            ],
          ),
        ),
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Date Header Banner (Sticky styling)
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainerLowest,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: ObsidianColors.border),
          ),
          child: Row(
            children: [
              const Icon(Icons.calendar_today_rounded,
                  size: 16, color: ObsidianColors.primary),
              const SizedBox(width: 8),
              Text('Today — Recorded Pipeline Events',
                  style: ObsidianTypography.titleSm()),
              const Spacer(),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('${events.length} events logged',
                    style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.outline)),
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),

        // Timeline items with vertical guide line
        ListView.separated(
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          itemCount: events.length,
          separatorBuilder: (_, __) => const SizedBox(height: 8),
          itemBuilder: (context, index) {
            final event = events[index];
            return _buildTimelineEventCard(context, event);
          },
        ),
      ],
    );
  }

  Widget _buildTimelineEventCard(BuildContext context, ActivityItem event) {
    Color levelColor = ObsidianColors.primary;
    String badgeTag = 'SCORER';
    IconData eventIcon = Icons.rule_rounded;

    if (event.level == ActivityLevel.error) {
      levelColor = ObsidianColors.error;
      badgeTag = 'ERROR';
      eventIcon = Icons.cloud_off_rounded;
    } else if (event.level == ActivityLevel.warn) {
      levelColor = ObsidianColors.warning;
      badgeTag = 'WARN';
      eventIcon = Icons.warning_amber_rounded;
    } else if (event.message.toLowerCase().contains('hardlink') ||
        event.message.toLowerCase().contains('moved') ||
        event.message.toLowerCase().contains('committed')) {
      levelColor = ObsidianColors.secondary;
      badgeTag = 'DISK_IO';
      eventIcon = Icons.folder_open_rounded;
    } else if (event.message.toLowerCase().contains('mal') ||
        event.message.toLowerCase().contains('metadata') ||
        event.message.toLowerCase().contains('sync')) {
      levelColor = ObsidianColors.tertiary;
      badgeTag = 'METADATA';
      eventIcon = Icons.sync_rounded;
    }

    // Try finding matching show poster if available
    String? posterUrl;
    if (event.malId != null) {
      final match =
          state.trackedShows.where((s) => s.id == event.malId).firstOrNull;
      posterUrl = match?.posterUrl;
    }

    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: event.level == ActivityLevel.error
            ? ObsidianColors.error.withOpacity(0.06)
            : ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: event.level == ActivityLevel.error
              ? ObsidianColors.error.withOpacity(0.3)
              : ObsidianColors.border,
        ),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // 2:3 Cover Poster or Icon
          if (posterUrl != null && posterUrl.isNotEmpty)
            Container(
              width: 40,
              height: 60,
              margin: const EdgeInsets.only(right: 12),
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(4),
                color: ObsidianColors.surfaceContainerHighest,
              ),
              clipBehavior: Clip.antiAlias,
              child: Image.network(
                posterUrl,
                fit: BoxFit.cover,
                errorBuilder: (_, __, ___) => const Icon(Icons.movie_rounded,
                    size: 20, color: ObsidianColors.outline),
              ),
            )
          else
            Container(
              width: 38,
              height: 38,
              margin: const EdgeInsets.only(right: 12),
              decoration: BoxDecoration(
                color: levelColor.withOpacity(0.12),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Icon(eventIcon, size: 20, color: levelColor),
            ),

          // Main text details
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text(event.formattedTime,
                        style: ObsidianTypography.monoCode(
                            color: ObsidianColors.outline)),
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(
                          horizontal: 6, vertical: 1.5),
                      decoration: BoxDecoration(
                        color: levelColor.withOpacity(0.18),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        badgeTag,
                        style: ObsidianTypography.labelBadge(color: levelColor),
                      ),
                    ),
                    const SizedBox(width: 8),
                    if (event.jobId != null)
                      Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 6, vertical: 1.5),
                        decoration: BoxDecoration(
                          color: ObsidianColors.surfaceContainerHighest,
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          'Job #${event.jobId}',
                          style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.onSurfaceVariant),
                        ),
                      ),
                  ],
                ),
                const SizedBox(height: 6),
                Text(
                  event.message,
                  style:
                      ObsidianTypography.bodyMd(color: ObsidianColors.onSurface),
                ),
              ],
            ),
          ),

          // Action button / telemetry
          const SizedBox(width: 12),
          Column(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLowest,
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Text(
                  event.level == ActivityLevel.error
                      ? 'ATTENTION'
                      : 'AUDIT #0${event.id}',
                  style: ObsidianTypography.labelBadge(
                    color: event.level == ActivityLevel.error
                        ? ObsidianColors.error
                        : ObsidianColors.outline,
                  ),
                ),
              ),
              const SizedBox(height: 8),
              if (event.level == ActivityLevel.error)
                TextButton.icon(
                  onPressed: () => state.refreshAll(),
                  icon: const Icon(Icons.refresh_rounded,
                      size: 14, color: ObsidianColors.error),
                  label: Text('Retry',
                      style: ObsidianTypography.labelBadge(
                          color: ObsidianColors.error)),
                  style: TextButton.styleFrom(
                    padding:
                        const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                    minimumSize: Size.zero,
                    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  ),
                )
              else
                TextButton.icon(
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(
                        content:
                            Text('Inspecting event details: ${event.message}'),
                        backgroundColor: ObsidianColors.surfaceContainerHigh,
                      ),
                    );
                  },
                  icon: const Icon(Icons.arrow_forward_rounded,
                      size: 14, color: ObsidianColors.primary),
                  label: Text('Inspect',
                      style: ObsidianTypography.labelBadge(
                          color: ObsidianColors.primary)),
                  style: TextButton.styleFrom(
                    padding:
                        const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                    minimumSize: Size.zero,
                    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  ),
                ),
            ],
          ),
        ],
      ),
    );
  }
}
