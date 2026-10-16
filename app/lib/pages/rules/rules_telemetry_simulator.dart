import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class RulesTelemetrySimulator extends StatelessWidget {
  final StateProvider state;

  const RulesTelemetrySimulator({super.key, required this.state});

  @override
  Widget build(BuildContext context) {
    final sampleTarget = state.queue.isNotEmpty
        ? state.queue.first.title
        : (state.shows.isNotEmpty
              ? '${state.shows.first.title} - 01'
              : 'No active test target');

    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
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
                'Policy Matcher Telemetry: ',
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.onSurface,
                ).copyWith(fontWeight: FontWeight.bold),
              ),
              Expanded(
                child: Text(
                  sampleTarget,
                  style: ObsidianTypography.monoCode(
                    color: ObsidianColors.outline,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              Text(
                'Calculated Score: +185 pts',
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.tertiary,
                ),
              ),
              const SizedBox(width: 12),
              Text(
                'Verdict: AUTO_COMMITTED_QBTT',
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.primary,
                ).copyWith(fontWeight: FontWeight.bold),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 6,
            runSpacing: 4,
            children: [
              _buildTelemetryChip('Group: SubsPlease (+50)'),
              _buildTelemetryChip('Res: 1080p (+40)'),
              _buildTelemetryChip('Codec: H.264/AVC (+20)'),
              _buildTelemetryChip('Audio: AAC 2.0 (+15)'),
              _buildTelemetryChip('Swarm: 142 Seeds (>10 Target, +60)'),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  'Inbox: BYPASSED',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.tertiary,
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildTelemetryChip(String text) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        text,
        style: ObsidianTypography.labelBadge(
          color: ObsidianColors.onSurfaceVariant,
        ),
      ),
    );
  }
}
