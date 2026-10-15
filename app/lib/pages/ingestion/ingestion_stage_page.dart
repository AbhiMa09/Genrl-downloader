import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'ingestion_context_strip.dart';
import 'ingestion_radar_panel.dart';
import 'pattern_card_a.dart';
import 'pattern_card_b.dart';
import 'pattern_card_c.dart';
import 'pattern_card_d.dart';

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
    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 20),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 1200),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // 1. Top System Context Strip
              const IngestionContextStrip(),
              const SizedBox(height: 20),

              // 2. Centered Hero Sync Stage with CyberIngestionLoader
              IngestionRadarPanel(
                isIngesting: _isIngesting,
                onToggleIngesting: () =>
                    setState(() => _isIngesting = !_isIngesting),
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
                      PatternCardA(),
                      // Card B: Skeleton & Shimmer Card
                      PatternCardB(),
                      // Card C: Packet Buffer & CRC32 Hash
                      PatternCardC(),
                      // Card D: Daemon Reconnect Modal
                      PatternCardD(),
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
