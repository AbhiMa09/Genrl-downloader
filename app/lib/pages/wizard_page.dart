import 'package:flutter/material.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';

class WizardPage extends StatefulWidget {
  final VoidCallback onComplete;

  const WizardPage({super.key, required this.onComplete});

  @override
  State<WizardPage> createState() => _WizardPageState();
}

class _WizardPageState extends State<WizardPage> {
  int _currentStep = 0;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: ObsidianColors.surface,
      body: Center(
        child: Container(
          width: 600,
          padding: const EdgeInsets.all(32),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainerLow,
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: ObsidianColors.border, width: 1),
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // Header
              Row(
                children: [
                  Container(
                    width: 36,
                    height: 36,
                    decoration: BoxDecoration(
                      color: ObsidianColors.primaryContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: const Icon(Icons.bolt_rounded,
                        color: ObsidianColors.onPrimaryContainer),
                  ),
                  const SizedBox(width: 14),
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('Welcome to Genrl Downloader',
                          style: ObsidianTypography.headlineMd()),
                      Text('First-Run Setup Guide',
                          style: ObsidianTypography.monoCode(
                              color: ObsidianColors.outline)),
                    ],
                  ),
                ],
              ),
              const SizedBox(height: 24),
              const Divider(color: ObsidianColors.border),
              const SizedBox(height: 20),

              if (_currentStep == 0) ...[
                Text('Step 1: Introduction', style: ObsidianTypography.titleSm()),
                const SizedBox(height: 10),
                Text(
                  'Genrl Downloader synchronizes your MyAnimeList watching list with torrent trackers and your local disk.\n\n'
                  'Before starting the automation loop, we will verify your qBittorrent client, set your folder locations, and link your MAL account.',
                  style: ObsidianTypography.bodyMd(),
                ),
              ] else if (_currentStep == 1) ...[
                Text('Step 2: qBittorrent Connection',
                    style: ObsidianTypography.titleSm()),
                const SizedBox(height: 10),
                Text(
                  'Genrl Downloader orchestrates torrents through qBittorrent WebUI. Ensure WebUI is enabled in qBittorrent Preferences -> Web UI.',
                  style: ObsidianTypography.bodyMd(),
                ),
                const SizedBox(height: 16),
                Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLowest,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Row(
                    children: [
                      const Icon(Icons.check_circle_rounded,
                          color: ObsidianColors.success, size: 20),
                      const SizedBox(width: 10),
                      Text('qBittorrent v5.2.4 detected on 127.0.0.1:8080',
                          style: ObsidianTypography.monoCode(
                              color: ObsidianColors.onSurface)),
                    ],
                  ),
                ),
              ] else if (_currentStep == 2) ...[
                Text('Step 3: Directory Configuration',
                    style: ObsidianTypography.titleSm()),
                const SizedBox(height: 10),
                Text(
                  'Select where incomplete torrents should stage, and where organized shows with synopsis.txt and poster icons will live.',
                  style: ObsidianTypography.bodyMd(),
                ),
                const SizedBox(height: 16),
                Text('Download staging: /home/genrl/Downloads',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline)),
                const SizedBox(height: 6),
                Text('Library folder:    /mnt/anime',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.primary)),
              ] else ...[
                Text('Step 4: MyAnimeList & First Sync',
                    style: ObsidianTypography.titleSm()),
                const SizedBox(height: 10),
                Text(
                  'The first sync presents a queue preview before anything downloads, capped at 25 torrents.\n\n'
                  'You can connect your MAL account now or proceed with local library scanning.',
                  style: ObsidianTypography.bodyMd(),
                ),
              ],

              const SizedBox(height: 32),
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  if (_currentStep > 0)
                    TextButton(
                      onPressed: () => setState(() => _currentStep--),
                      child: Text('Back', style: ObsidianTypography.bodySm()),
                    ),
                  const SizedBox(width: 8),
                  ElevatedButton(
                    onPressed: () {
                      if (_currentStep < 3) {
                        setState(() => _currentStep++);
                      } else {
                        widget.onComplete();
                      }
                    },
                    style: ElevatedButton.styleFrom(
                      backgroundColor: ObsidianColors.primaryContainer,
                      foregroundColor: ObsidianColors.onPrimaryContainer,
                      padding: const EdgeInsets.symmetric(
                          horizontal: 20, vertical: 12),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(8),
                      ),
                    ),
                    child: Text(
                      _currentStep == 3 ? 'Finish & Launch' : 'Continue',
                      style: ObsidianTypography.titleSm(),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
