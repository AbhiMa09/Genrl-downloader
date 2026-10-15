import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../widgets/cyber_loader.dart';
import 'ingestion_meta_chips.dart';

class IngestionRadarPanel extends StatelessWidget {
  final bool isIngesting;
  final VoidCallback onToggleIngesting;

  const IngestionRadarPanel({
    super.key,
    required this.isIngesting,
    required this.onToggleIngesting,
  });

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    return Container(
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: ObsidianColors.border, width: 0.5),
        boxShadow: [
          BoxShadow(
            color: ObsidianColors.surfaceContainerLowest.withOpacity(0.8),
            blurRadius: 24,
            offset: const Offset(0, 8),
          ),
        ],
      ),
      padding: const EdgeInsets.all(28),
      child: Column(
        children: [
          // Micro Status Header Bar
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(Icons.hub_rounded,
                      size: 16, color: ObsidianColors.tertiary),
                  const SizedBox(width: 6),
                  Text(
                    'TASK_ID: ingest-202410-ep482',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ],
              ),
              Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: 6,
                      height: 6,
                      decoration: const BoxDecoration(
                        color: ObsidianColors.primary,
                        shape: BoxShape.circle,
                      ),
                    ),
                    const SizedBox(width: 6),
                    Text(
                      'HIGH PRIORITY DISPATCH',
                      style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.outline,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 20),

          // The Animated Cybernetic Ingestion Radar
          Stack(
            alignment: Alignment.center,
            children: [
              Container(
                width: 220,
                height: 220,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  boxShadow: [
                    BoxShadow(
                      color: ObsidianColors.primary.withOpacity(0.12),
                      blurRadius: 40,
                      spreadRadius: 10,
                    ),
                  ],
                ),
              ),
              CyberIngestionLoader(
                size: 220,
                animate: isIngesting,
              ),
            ],
          ),
          const SizedBox(height: 20),

          // Title & Description
          Text(
            'Resolving Swarm Manifests & Hashing Blocks...',
            style: ObsidianTypography.headlineLg(),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 6),
          Text(
            'Verifying candidate torrent chunks against MAL episode schedule and AnimeTosho RSS.',
            style: ObsidianTypography.bodyMd(
              color: ObsidianColors.onSurfaceVariant,
            ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 14),

          // Monospace Telemetry Stream Badge
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: ObsidianColors.border, width: 0.5),
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(Icons.refresh_rounded,
                    size: 16, color: ObsidianColors.tertiary),
                const SizedBox(width: 8),
                Text(
                  'Thread #4 [libtorrent 2.0.9] • 42.8 MB/s • Peer handshake: 84/120 OK',
                  style: ObsidianTypography.monoCode(
                    color: ObsidianColors.primary,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // Dual-Segmented Progress Metrics
          ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 580),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(
                      '146 / 214 episodes verified',
                      style: ObsidianTypography.monoMetric(
                        color: ObsidianColors.onSurface,
                      ),
                    ),
                    Text(
                      '68.2%',
                      style: ObsidianTypography.monoMetric(
                        color: ObsidianColors.primary,
                      ).copyWith(fontWeight: FontWeight.bold),
                    ),
                  ],
                ),
                const SizedBox(height: 8),

                // Dual Segment Bar
                Container(
                  height: 10,
                  padding: const EdgeInsets.all(2),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Row(
                    children: [
                      // Segment 1: Completed / Hardlinked (54%)
                      Expanded(
                        flex: 54,
                        child: Container(
                          decoration: BoxDecoration(
                            color: ObsidianColors.primary,
                            borderRadius: BorderRadius.circular(6),
                            boxShadow: [
                              BoxShadow(
                                color: ObsidianColors.primary.withOpacity(0.6),
                                blurRadius: 8,
                              ),
                            ],
                          ),
                        ),
                      ),
                      const SizedBox(width: 2),
                      // Segment 2: Hashing Active (14%)
                      Expanded(
                        flex: 14,
                        child: Container(
                          decoration: BoxDecoration(
                            color: ObsidianColors.tertiaryContainer,
                            borderRadius: BorderRadius.circular(6),
                          ),
                        ),
                      ),
                      const SizedBox(width: 2),
                      // Remaining (32%)
                      const Expanded(
                        flex: 32,
                        child: SizedBox(),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 8),

                Wrap(
                  alignment: WrapAlignment.spaceBetween,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  spacing: 12,
                  runSpacing: 6,
                  children: [
                    Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Container(
                          width: 8,
                          height: 8,
                          decoration: const BoxDecoration(
                            color: ObsidianColors.primary,
                            shape: BoxShape.circle,
                          ),
                        ),
                        const SizedBox(width: 5),
                        Text(
                          'Committed: 115 Eps',
                          style: ObsidianTypography.monoCode(
                            color: ObsidianColors.outline,
                            fontSize: 11,
                          ),
                        ),
                        const SizedBox(width: 12),
                        Container(
                          width: 8,
                          height: 8,
                          decoration: const BoxDecoration(
                            color: ObsidianColors.tertiaryContainer,
                            shape: BoxShape.circle,
                          ),
                        ),
                        const SizedBox(width: 5),
                        Text(
                          'Chunk Hash: 31 Eps',
                          style: ObsidianTypography.monoCode(
                            color: ObsidianColors.outline,
                            fontSize: 11,
                          ),
                        ),
                      ],
                    ),
                    Text(
                      'Time remaining: ~24s',
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline,
                        fontSize: 11,
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // Step Progress Pipeline (4 Steps)
          ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 680),
            child: const Wrap(
              spacing: 8,
              runSpacing: 8,
              alignment: WrapAlignment.center,
              children: [
                IngestionPipelineStep(
                  icon: Icons.check_circle_rounded,
                  iconColor: ObsidianColors.tertiary,
                  stepTitle: '1. Handshake',
                  detail: 'Tracker OK (200)',
                  isActive: false,
                ),
                IngestionPipelineStep(
                  icon: Icons.check_circle_rounded,
                  iconColor: ObsidianColors.tertiary,
                  stepTitle: '2. Swarm Scrape',
                  detail: '84 Peers Found',
                  isActive: false,
                ),
                IngestionPipelineStep(
                  icon: Icons.cyclone_rounded,
                  iconColor: ObsidianColors.secondary,
                  stepTitle: '3. Ingestion & Link',
                  detail: 'Active Hardlink',
                  isActive: true,
                ),
                IngestionPipelineStep(
                  icon: Icons.pending_outlined,
                  iconColor: ObsidianColors.outline,
                  stepTitle: '4. DB Commit',
                  detail: 'Queued',
                  isActive: false,
                  dimmed: true,
                ),
              ],
            ),
          ),
          const SizedBox(height: 28),

          // Action Controls
          Wrap(
            spacing: 12,
            runSpacing: 10,
            alignment: WrapAlignment.center,
            children: [
              OutlinedButton.icon(
                onPressed: onToggleIngesting,
                icon: Icon(
                  isIngesting
                      ? Icons.pause_rounded
                      : Icons.play_arrow_rounded,
                  size: 16,
                  color: ObsidianColors.outline,
                ),
                label: Text(
                  isIngesting ? 'Pause Ingestion' : 'Resume Ingestion',
                  style: ObsidianTypography.bodySm(),
                ),
                style: OutlinedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainerHighest,
                  foregroundColor: ObsidianColors.onSurface,
                  side: const BorderSide(
                      color: ObsidianColors.border, width: 0.5),
                  padding: const EdgeInsets.symmetric(
                      horizontal: 16, vertical: 12),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                ),
              ),
              ElevatedButton.icon(
                onPressed: () => state.selectPage(0),
                icon: const Icon(Icons.terminal_rounded, size: 16),
                label: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Text('Background Daemon'),
                    const SizedBox(width: 6),
                    Container(
                      padding: const EdgeInsets.symmetric(
                          horizontal: 5, vertical: 1.5),
                      decoration: BoxDecoration(
                        color: ObsidianColors.onPrimary.withOpacity(0.2),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        'Ctrl+B',
                        style: ObsidianTypography.monoCode(
                          color: ObsidianColors.onPrimary,
                        ),
                      ),
                    ),
                  ],
                ),
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.primary,
                  foregroundColor: ObsidianColors.onPrimary,
                  elevation: 0,
                  padding: const EdgeInsets.symmetric(
                      horizontal: 18, vertical: 12),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                  textStyle: ObsidianTypography.titleSm(),
                ),
              ),
              OutlinedButton.icon(
                onPressed: () => state.selectPage(5), // Activity page
                icon: const Icon(Icons.monitor_heart_rounded,
                    size: 16, color: ObsidianColors.tertiary),
                label: Text(
                  'View Live Telemetry',
                  style: ObsidianTypography.bodySm(),
                ),
                style: OutlinedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainerHighest,
                  foregroundColor: ObsidianColors.onSurface,
                  side: const BorderSide(
                      color: ObsidianColors.border, width: 0.5),
                  padding: const EdgeInsets.symmetric(
                      horizontal: 16, vertical: 12),
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
}
