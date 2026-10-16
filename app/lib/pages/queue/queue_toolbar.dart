import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class QueueToolbar extends StatelessWidget {
  final StateProvider state;
  final int count;

  const QueueToolbar({super.key, required this.state, required this.count});

  @override
  Widget build(BuildContext context) {
    final status = state.status;

    return Container(
      padding: const EdgeInsets.fromLTRB(24, 16, 24, 12),
      decoration: const BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        border: Border(bottom: BorderSide(color: ObsidianColors.border)),
      ),
      child: Wrap(
        alignment: WrapAlignment.spaceBetween,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: 16,
        runSpacing: 10,
        children: [
          // Left: Title & Quick Swarm Telemetry
          Wrap(
            crossAxisAlignment: WrapCrossAlignment.center,
            spacing: 12,
            runSpacing: 6,
            children: [
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    'Ingestion Queue',
                    style: ObsidianTypography.headlineMd(),
                  ),
                  const SizedBox(width: 8),
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 2,
                    ),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainer,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      '$count active jobs',
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.onSurfaceVariant,
                        fontSize: 11,
                      ),
                    ),
                  ),
                ],
              ),
              Container(width: 1, height: 14, color: ObsidianColors.border),
              Wrap(
                spacing: 8,
                children: [
                  _buildPill(
                    'DL: ',
                    status.downloadSpeedFormatted,
                    ObsidianColors.primary,
                  ),
                  _buildPill(
                    'UL: ',
                    status.uploadSpeedFormatted,
                    ObsidianColors.tertiary,
                  ),
                  _buildPill(
                    'Free Space: ',
                    '4.58 TB',
                    ObsidianColors.onSurfaceVariant,
                  ),
                ],
              ),
            ],
          ),

          // Right: Global Actions
          Wrap(
            spacing: 8,
            runSpacing: 6,
            children: [
              OutlinedButton.icon(
                onPressed: () => state.toggleAutomation(),
                icon: Icon(
                  state.status.automationOn
                      ? Icons.pause_rounded
                      : Icons.play_arrow_rounded,
                  size: 14,
                  color: ObsidianColors.outline,
                ),
                label: Text(
                  state.status.automationOn ? 'Pause Queue' : 'Resume Queue',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurface,
                    fontSize: 11,
                  ),
                ),
                style: OutlinedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainer,
                  side: const BorderSide(color: ObsidianColors.border),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 8,
                  ),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                ),
              ),
              OutlinedButton.icon(
                onPressed: () => state.refreshAll(),
                icon: const Icon(
                  Icons.restart_alt_rounded,
                  size: 14,
                  color: ObsidianColors.tertiary,
                ),
                label: Text(
                  'Retry Failed',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurface,
                    fontSize: 11,
                  ),
                ),
                style: OutlinedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainer,
                  side: const BorderSide(color: ObsidianColors.border),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 8,
                  ),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                ),
              ),
              ElevatedButton.icon(
                onPressed: () => state.selectPage(4),
                icon: const Icon(Icons.add_rounded, size: 16),
                label: Text(
                  'Manual Magnet',
                  style: ObsidianTypography.titleSm(
                    color: ObsidianColors.onPrimary,
                    fontSize: 12,
                  ),
                ),
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.primary,
                  foregroundColor: ObsidianColors.onPrimary,
                  elevation: 0,
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 8,
                  ),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildPill(String label, String value, Color color) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer.withOpacity(0.7),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            label,
            style: ObsidianTypography.monoCode(
              color: ObsidianColors.outline,
              fontSize: 10,
            ),
          ),
          Text(
            value,
            style: ObsidianTypography.monoMetric(
              color: color,
              fontWeight: FontWeight.bold,
              fontSize: 10,
            ),
          ),
        ],
      ),
    );
  }
}
