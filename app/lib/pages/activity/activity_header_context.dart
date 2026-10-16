import 'package:flutter/material.dart';

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
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(
                    content: Text(
                      'Audit log exported to /tmp/genrl-activity.log',
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
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(
                    content: Text('Logs older than 30 days purged'),
                    backgroundColor: ObsidianColors.surfaceContainerHigh,
                  ),
                );
              },
              icon: const Icon(
                Icons.delete_sweep_rounded,
                size: 16,
                color: ObsidianColors.outline,
              ),
              label: const Text('Purge > 30d'),
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
