import 'package:flutter/material.dart';

import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/loading/cyber_loader.dart';
import '../widgets/loading/inline_spinner.dart';
import '../widgets/loading/skeleton_poster_card.dart';
import '../widgets/loading/connection_banner.dart';
import '../widgets/loading/ingestion_progress.dart';

/// Debug-only gallery showcasing loading state primitives.
/// Hidden in production release builds.
class LoadingGalleryPage extends StatefulWidget {
  const LoadingGalleryPage({super.key});

  @override
  State<LoadingGalleryPage> createState() => _LoadingGalleryPageState();
}

class _LoadingGalleryPageState extends State<LoadingGalleryPage> {
  IngestionStage _activeStage = IngestionStage.metadata;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: ObsidianColors.surface,
      appBar: AppBar(
        title: const Text('Loading Primitives Gallery [DEBUG ONLY]'),
        backgroundColor: ObsidianColors.surfaceContainerLow,
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 1000),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                // 1. Connection Banner
                const Text(
                  '1. Connection Banner Primitive',
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
                const SizedBox(height: 8),
                const ConnectionBanner(),
                const SizedBox(height: 24),

                // 2. CyberLoader & InlineSpinner
                const Text(
                  '2. CyberLoader (Concentric Rings) & Inline Spinner',
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
                const SizedBox(height: 12),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceAround,
                  children: [
                    const CyberLoader(size: 160),
                    Column(
                      children: [
                        const InlineSpinner(size: 24),
                        const SizedBox(height: 8),
                        Text(
                          'Inline Spinner',
                          style: ObsidianTypography.monoCode(),
                        ),
                      ],
                    ),
                    const SizedBox(
                      width: 120,
                      height: 180,
                      child: SkeletonPosterCard(),
                    ),
                  ],
                ),
                const SizedBox(height: 32),

                // 3. IngestionProgress Component
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    const Text(
                      '3. IngestionProgress Component',
                      style: TextStyle(fontWeight: FontWeight.bold),
                    ),
                    DropdownButton<IngestionStage>(
                      value: _activeStage,
                      dropdownColor: ObsidianColors.surfaceContainerHigh,
                      items: IngestionStage.values.map((s) {
                        return DropdownMenuItem(
                          value: s,
                          child: Text(
                            s.name,
                            style: ObsidianTypography.bodySm(),
                          ),
                        );
                      }).toList(),
                      onChanged: (val) {
                        if (val != null) setState(() => _activeStage = val);
                      },
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                Container(
                  padding: const EdgeInsets.all(20),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(16),
                    border: Border.all(color: ObsidianColors.border),
                  ),
                  child: IngestionProgress(
                    stage: _activeStage,
                    processedCount: 146,
                    totalCount: 214,
                    etaSeconds: 24,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
