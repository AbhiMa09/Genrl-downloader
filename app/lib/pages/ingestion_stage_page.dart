import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/cyber_loader.dart';

/// Ingestion & Swarm Telemetry Page (Web & Desktop Dashboard Stage).
/// Visualizes live packet hashing, tracker handshakes, swarm block verification,
/// dual-segmented progress metrics, and the standardized loading state patterns.
class IngestionStagePage extends StatefulWidget {
  const IngestionStagePage({super.key});

  @override
  State<IngestionStagePage> createState() => _IngestionStagePageState();
}

class _IngestionStagePageState extends State<IngestionStagePage> {
  bool _isIngesting = true;

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 20),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 1200),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // 1. Top System Context Strip
              Container(
                padding: const EdgeInsets.all(14),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: ObsidianColors.border, width: 0.5),
                ),
                child: Wrap(
                  alignment: WrapAlignment.spaceBetween,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  spacing: 16,
                  runSpacing: 10,
                  children: [
                    Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Container(
                          width: 10,
                          height: 10,
                          decoration: const BoxDecoration(
                            color: ObsidianColors.tertiary,
                            shape: BoxShape.circle,
                          ),
                        ),
                        const SizedBox(width: 12),
                        Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                Text(
                                  'Syncing & Ingestion Daemon',
                                  style: ObsidianTypography.titleSm(),
                                ),
                                const SizedBox(width: 8),
                                Container(
                                  padding: const EdgeInsets.symmetric(
                                      horizontal: 6, vertical: 2),
                                  decoration: BoxDecoration(
                                    color: ObsidianColors.tertiaryContainer
                                        .withOpacity(0.3),
                                    borderRadius: BorderRadius.circular(4),
                                    border: Border.all(
                                      color: ObsidianColors.tertiary
                                          .withOpacity(0.3),
                                      width: 0.5,
                                    ),
                                  ),
                                  child: Text(
                                    'RPC ACTIVE',
                                    style: ObsidianTypography.labelBadge(
                                      color: ObsidianColors.tertiary,
                                    ),
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 2),
                            Text(
                              'Session PID: 40921 • IPC: /run/genrl/daemon.sock',
                              style: ObsidianTypography.monoCode(
                                color: ObsidianColors.outline,
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),

                    Wrap(
                      spacing: 8,
                      runSpacing: 6,
                      children: [
                        const _MetaChip(text: 'Worker Threads: 08'),
                        const _MetaChip(text: 'qBt 4.6.2'),
                        Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 8, vertical: 4),
                          decoration: BoxDecoration(
                            color: ObsidianColors.secondaryContainer
                                .withOpacity(0.4),
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Text(
                            'V2.4.1-STABLE',
                            style: ObsidianTypography.monoCode(
                              color: ObsidianColors.secondary,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 20),

              // 2. Centered Hero Sync Stage with CyberIngestionLoader
              Container(
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(16),
                  border: Border.all(color: ObsidianColors.border, width: 0.5),
                  boxShadow: [
                    BoxShadow(
                      color: ObsidianColors.surfaceContainerLowest
                          .withOpacity(0.8),
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
                          padding: const EdgeInsets.symmetric(
                              horizontal: 8, vertical: 3),
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
                          animate: _isIngesting,
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
                      padding: const EdgeInsets.symmetric(
                          horizontal: 14, vertical: 6),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerLowest,
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(
                            color: ObsidianColors.border, width: 0.5),
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
                                          color: ObsidianColors.primary
                                              .withOpacity(0.6),
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
                          _PipelineStep(
                            icon: Icons.check_circle_rounded,
                            iconColor: ObsidianColors.tertiary,
                            stepTitle: '1. Handshake',
                            detail: 'Tracker OK (200)',
                            isActive: false,
                          ),
                          _PipelineStep(
                            icon: Icons.check_circle_rounded,
                            iconColor: ObsidianColors.tertiary,
                            stepTitle: '2. Swarm Scrape',
                            detail: '84 Peers Found',
                            isActive: false,
                          ),
                          _PipelineStep(
                            icon: Icons.cyclone_rounded,
                            iconColor: ObsidianColors.secondary,
                            stepTitle: '3. Ingestion & Link',
                            detail: 'Active Hardlink',
                            isActive: true,
                          ),
                          _PipelineStep(
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
                          onPressed: () {
                            setState(() => _isIngesting = !_isIngesting);
                          },
                          icon: Icon(
                            _isIngesting
                                ? Icons.pause_rounded
                                : Icons.play_arrow_rounded,
                            size: 16,
                            color: ObsidianColors.outline,
                          ),
                          label: Text(
                            _isIngesting ? 'Pause Ingestion' : 'Resume Ingestion',
                            style: ObsidianTypography.bodySm(),
                          ),
                          style: OutlinedButton.styleFrom(
                            backgroundColor:
                                ObsidianColors.surfaceContainerHighest,
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
                                  color: ObsidianColors.onPrimary
                                      .withOpacity(0.2),
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
                            backgroundColor:
                                ObsidianColors.surfaceContainerHighest,
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
              ),
              const SizedBox(height: 32),

              // 3. Application Loading State Patterns Showcase Grid
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Application Loading State Patterns',
                          style: ObsidianTypography.headlineMd(),
                        ),
                        const SizedBox(height: 4),
                        Text(
                          'Standardized feedback primitives deployed across Genrl Downloader runtime views.',
                          style: ObsidianTypography.bodySm(
                              color: ObsidianColors.outline),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 16),
                  Text(
                    'PATTERN LIB // TOKENS v2',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 16),

              // 4 Pattern Cards Grid
              LayoutBuilder(
                builder: (context, constraints) {
                  final isWide = constraints.maxWidth > 900;
                  final isMedium = constraints.maxWidth > 600;

                  int crossAxisCount = 1;
                  if (isWide) {
                    crossAxisCount = 4;
                  } else if (isMedium) {
                    crossAxisCount = 2;
                  }

                  return GridView.count(
                    crossAxisCount: crossAxisCount,
                    shrinkWrap: true,
                    physics: const NeverScrollableScrollPhysics(),
                    crossAxisSpacing: 14,
                    mainAxisSpacing: 14,
                    childAspectRatio: isWide ? 0.85 : 0.82,
                    children: const [
                      // Card A: Mini Inline Spinner
                      _PatternCardA(),
                      // Card B: Skeleton & Shimmer Card
                      _PatternCardB(),
                      // Card C: Packet Buffer & CRC32 Hash
                      _PatternCardC(),
                      // Card D: Daemon Reconnect Modal
                      _PatternCardD(),
                    ],
                  );
                },
              ),
              const SizedBox(height: 24),
            ],
          ),
        ),
      ),
    );
  }
}

class _MetaChip extends StatelessWidget {
  final String text;

  const _MetaChip({required this.text});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        text,
        style: ObsidianTypography.monoCode(
          color: ObsidianColors.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _PipelineStep extends StatelessWidget {
  final IconData icon;
  final Color iconColor;
  final String stepTitle;
  final String detail;
  final bool isActive;
  final bool dimmed;

  const _PipelineStep({
    required this.icon,
    required this.iconColor,
    required this.stepTitle,
    required this.detail,
    this.isActive = false,
    this.dimmed = false,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 154,
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: isActive
            ? ObsidianColors.secondaryContainer.withOpacity(0.3)
            : ObsidianColors.surfaceContainerHigh.withOpacity(dimmed ? 0.6 : 1),
        borderRadius: BorderRadius.circular(8),
        border: isActive
            ? Border.all(
                color: ObsidianColors.secondary.withOpacity(0.5), width: 1)
            : null,
      ),
      child: Row(
        children: [
          Icon(icon, size: 16, color: iconColor),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  stepTitle,
                  style: ObsidianTypography.labelBadge(
                    color: isActive
                        ? ObsidianColors.secondary
                        : ObsidianColors.onSurface,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
                Text(
                  detail,
                  style: ObsidianTypography.monoCode(
                    color: isActive
                        ? ObsidianColors.onSurface
                        : ObsidianColors.outline,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// Pattern Card A: Mini Inline Spinner
class _PatternCardA extends StatelessWidget {
  const _PatternCardA();

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border, width: 0.5),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text('Mini Inline Spinner',
                    style: ObsidianTypography.titleSm(),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('TABLE / ROW',
                    style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.outline)),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            'Compressed feedback when re-indexing season folders or triggering single item hash checks.',
            style: ObsidianTypography.bodySm(
                color: ObsidianColors.onSurfaceVariant),
          ),
          const Spacer(),

          // Showcase Canvas
          Container(
            padding: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              children: [
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHigh,
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: Row(
                    children: [
                      const SizedBox(
                        width: 14,
                        height: 14,
                        child: CircularProgressIndicator(
                          strokeWidth: 2,
                          valueColor: AlwaysStoppedAnimation<Color>(
                              ObsidianColors.tertiary),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          'Ingestion Job #101 [1080p HEVC]',
                          style: ObsidianTypography.monoCode(),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      Text('POLL',
                          style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.tertiary)),
                    ],
                  ),
                ),
                const SizedBox(height: 6),
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer.withOpacity(0.4),
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: Row(
                    children: [
                      const Icon(Icons.check,
                          size: 14, color: ObsidianColors.outline),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          'Ingestion Job #100 [BDRip FLAC]',
                          style: ObsidianTypography.monoCode(
                              color: ObsidianColors.outline),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      Text('IDLE',
                          style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.outline)),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 10),

          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Flexible(
                child: Text('Latency: <4ms',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 4),
              Flexible(
                child: Text('CSS Keyframe',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.tertiary, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

/// Pattern Card B: Skeleton & Shimmer Card
class _PatternCardB extends StatelessWidget {
  const _PatternCardB();

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border, width: 0.5),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text('Skeleton & Shimmer Card',
                    style: ObsidianTypography.titleSm(),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('POSTER GRID',
                    style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.outline)),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            'Used when fetching remote posters from AniList and parsing MAL seasonal synopses.',
            style: ObsidianTypography.bodySm(
                color: ObsidianColors.onSurfaceVariant),
          ),
          const Spacer(),

          // Showcase Canvas (2:3 preview dummy)
          Container(
            padding: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            alignment: Alignment.center,
            child: Container(
              width: 72,
              height: 100,
              padding: const EdgeInsets.all(6),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerHigh,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.end,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Container(
                    width: 36,
                    height: 6,
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceBright,
                      borderRadius: BorderRadius.circular(2),
                    ),
                  ),
                  const SizedBox(height: 4),
                  Container(
                    width: 50,
                    height: 8,
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceBright,
                      borderRadius: BorderRadius.circular(2),
                    ),
                  ),
                  const SizedBox(height: 4),
                  Container(
                    width: 24,
                    height: 6,
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceBright,
                      borderRadius: BorderRadius.circular(2),
                    ),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 10),

          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Flexible(
                child: Text('Ratio: 2:3 Portrait',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 4),
              Flexible(
                child: Text('Shimmer Pass',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.secondary, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

/// Pattern Card C: Packet Buffer & CRC32 Hash
class _PatternCardC extends StatelessWidget {
  const _PatternCardC();

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border, width: 0.5),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text('Packet Buffer Check',
                    style: ObsidianTypography.titleSm(),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                decoration: BoxDecoration(
                  color:
                      ObsidianColors.secondaryContainer.withOpacity(0.4),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('TELEMETRY',
                    style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.secondary)),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            'Radar telemetry loader for CRC32 / MD5 block integrity while writing to /mnt/anime.',
            style: ObsidianTypography.bodySm(
                color: ObsidianColors.onSurfaceVariant),
          ),
          const Spacer(),

          // Showcase Canvas
          Container(
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              children: [
                Container(
                  width: 44,
                  height: 44,
                  decoration: const BoxDecoration(
                    color: ObsidianColors.surfaceContainerHigh,
                    shape: BoxShape.circle,
                  ),
                  child: const Icon(
                    Icons.radar_rounded,
                    color: ObsidianColors.primary,
                    size: 24,
                  ),
                ),
                const SizedBox(height: 6),
                Text(
                  'CRC32: 0x9B4E1A • 85.4 MB/s',
                  style: ObsidianTypography.monoCode(
                    color: ObsidianColors.primary,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 10),

          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Flexible(
                child: Text('Buffer: 64MB In-RAM',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 4),
              Flexible(
                child: Text('Zero-Drop IO',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.primary, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

/// Pattern Card D: Daemon Ping Reconnect
class _PatternCardD extends StatelessWidget {
  const _PatternCardD();

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border, width: 0.5),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text('Daemon Ping Reconnect',
                    style: ObsidianTypography.titleSm(),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.errorContainer.withOpacity(0.4),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('RPC RETRY',
                    style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.error)),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            'Handshake monitor when qBittorrent WebUI reloads or unix socket restarts.',
            style: ObsidianTypography.bodySm(
                color: ObsidianColors.onSurfaceVariant),
          ),
          const Spacer(),

          // Showcase Canvas
          Container(
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Row(
                      children: [
                        Container(
                          width: 6,
                          height: 6,
                          decoration: const BoxDecoration(
                            color: ObsidianColors.tertiary,
                            shape: BoxShape.circle,
                          ),
                        ),
                        const SizedBox(width: 6),
                        Text(
                          '127.0.0.1:8080',
                          style: ObsidianTypography.monoCode(fontSize: 10),
                        ),
                      ],
                    ),
                    Text(
                      '12 ms',
                      style: ObsidianTypography.monoMetric(
                        color: ObsidianColors.tertiary,
                        fontSize: 10,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                ClipRRect(
                  borderRadius: BorderRadius.circular(4),
                  child: const LinearProgressIndicator(
                    value: 0.82,
                    backgroundColor: ObsidianColors.surfaceContainerHighest,
                    valueColor: AlwaysStoppedAnimation<Color>(
                        ObsidianColors.tertiary),
                    minHeight: 5,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 10),

          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Flexible(
                child: Text('Heartbeat: 2000ms',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 4),
              Flexible(
                child: Text('TCP_NODELAY',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.tertiary, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
