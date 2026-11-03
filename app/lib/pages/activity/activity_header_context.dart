import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class ActivityHeaderContext extends StatelessWidget {
  final StateProvider state;

  const ActivityHeaderContext({super.key, required this.state});

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 10,
                runSpacing: 4,
                children: [
                  Text(
                    'Activity & Decision Log',
                    style: ObsidianTypography.headlineLg(),
                  ),
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 2,
                    ),
                    decoration: BoxDecoration(
                      color: ObsidianColors.primary.withOpacity(0.12),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      'AUDIT STREAM',
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.primary,
                      ).copyWith(fontWeight: FontWeight.bold),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 4),
              Text(
                'Real-time pipeline decisions, scoring metrics, indexer queries, and hardlink telemetry.',
                style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
              ),
            ],
          ),
        ),
        Row(
          children: [
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLow,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: ObsidianColors.border),
              ),
              child: Row(
                children: [
                  Container(
                    width: 7,
                    height: 7,
                    decoration: const BoxDecoration(
                      shape: BoxShape.circle,
                      color: ObsidianColors.tertiary,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    'Auto-tail: ACTIVE',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.onSurface,
                    ),
                  ),
                  const SizedBox(width: 6),
                  Text(
                    '500ms',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 10),
            ElevatedButton.icon(
              onPressed: () {
                final exportStr = state.events
                    .map(
                      (e) =>
                          '[${e.timestamp.toIso8601String()}] [${e.level.name.toUpperCase()}] '
                          '${e.jobId != null ? "Job #${e.jobId} " : ""}'
                          '${e.message}',
                    )
                    .join('\n');
                Clipboard.setData(ClipboardData(text: exportStr));
                try {
                  File('/tmp/genrl-activity.log').writeAsStringSync(exportStr);
                } catch (_) {}
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: Text(
                      'Exported ${state.events.length} events to /tmp/genrl-activity.log & clipboard',
                    ),
                    backgroundColor: ObsidianColors.surfaceContainerHigh,
                  ),
                );
              },
              icon: const Icon(
                Icons.ios_share_rounded,
                size: 16,
                color: ObsidianColors.primary,
              ),
              label: const Text('Export'),
              style: ElevatedButton.styleFrom(
                backgroundColor: ObsidianColors.surfaceContainer,
                foregroundColor: ObsidianColors.onSurface,
                elevation: 0,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(8),
                  side: const BorderSide(color: ObsidianColors.border),
                ),
              ),
            ),
            const SizedBox(width: 8),
            OutlinedButton.icon(
              onPressed: () {
                state.refreshEvents();
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(
                    content: Text('Activity logs refreshed and synchronized'),
                    backgroundColor: ObsidianColors.surfaceContainerHigh,
                  ),
                );
              },
              icon: const Icon(
                Icons.refresh_rounded,
                size: 16,
                color: ObsidianColors.outline,
              ),
              label: const Text('Sync / Refresh'),
              style: OutlinedButton.styleFrom(
                side: const BorderSide(color: ObsidianColors.border),
                foregroundColor: ObsidianColors.onSurfaceVariant,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(8),
                ),
              ),
            ),
          ],
        ),
      ],
    );
  }
}
