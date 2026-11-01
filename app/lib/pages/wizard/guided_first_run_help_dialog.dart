import 'package:flutter/material.dart';

import '../../theme/colors.dart';
import '../../theme/typography.dart';

/// Modal dialog providing help and troubleshooting for the 5 most common setup issues.
class GuidedFirstRunHelpDialog extends StatelessWidget {
  const GuidedFirstRunHelpDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      builder: (ctx) => const GuidedFirstRunHelpDialog(),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      backgroundColor: ObsidianColors.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(16),
        side: const BorderSide(color: ObsidianColors.border),
      ),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 640, maxHeight: 600),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Row(
                    children: [
                      const Icon(
                        Icons.help_outline_rounded,
                        color: ObsidianColors.primary,
                        size: 22,
                      ),
                      const SizedBox(width: 10),
                      Text(
                        'Help & Troubleshooting',
                        style: ObsidianTypography.headlineMd(),
                      ),
                    ],
                  ),
                  IconButton(
                    icon: const Icon(Icons.close_rounded, size: 20),
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                ],
              ),
              const SizedBox(height: 16),
              const Divider(color: ObsidianColors.border, height: 1),
              const SizedBox(height: 16),
              SizedBox(
                height: 380,
                child: ListView(
                  shrinkWrap: true,
                  children: const [
                    _HelpItem(
                      number: '1',
                      title: 'Windows SmartScreen "Unrecognized App" Warning',
                      description: 'Because Genrl is an open-source community tool without an expensive commercial code-signing certificate, Windows SmartScreen may show a blue warning. Click "More info", then click "Run anyway" to proceed safely.',
                    ),
                    _HelpItem(
                      number: '2',
                      title: 'Antivirus False Positives or File Locking',
                      description: 'Some antivirus programs may temporarily lock files during hardlink operations or sqlite transactions. If you experience unexpected file locks, add "%APPDATA%\\Genrl Downloader" and your library directory to your antivirus exclusions.',
                    ),
                    _HelpItem(
                      number: '3',
                      title: 'qBittorrent Not Detected / Connection Refused',
                      description: 'Verify that qBittorrent is running. In qBittorrent, open Tools -> Options -> Web UI. Ensure "Web User Interface (Remote control)" is enabled, port matches (default 8080 or 18080), and "Bypass authentication for clients on localhost" is checked.',
                    ),
                    _HelpItem(
                      number: '4',
                      title: 'MyAnimeList Login Window Does Not Return',
                      description: 'After authorizing in your browser, the page should automatically redirect to http://127.0.0.1:21445/callback. If you use Brave Browser or aggressive ad blockers, disable Shields / protection on myanimelist.net during authorization so session cookies are preserved.',
                    ),
                    _HelpItem(
                      number: '5',
                      title: 'No Free Disk Space / Storage Warnings',
                      description: 'Anime video files require sufficient disk space. Ensure the drive where your Downloads and Library folders are located has at least 10 GB free space. Placing both folders on the same NTFS drive enables zero-cost hardlinking without taking double space.',
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 16),
              Align(
                alignment: Alignment.centerRight,
                child: TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: const Text('Got it'),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _HelpItem extends StatelessWidget {
  final String number;
  final String title;
  final String description;

  const _HelpItem({
    required this.number,
    required this.title,
    required this.description,
  });

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 16),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 24,
            height: 24,
            decoration: const BoxDecoration(
              color: ObsidianColors.surfaceContainerHigh,
              shape: BoxShape.circle,
            ),
            alignment: Alignment.center,
            child: Text(
              number,
              style: ObsidianTypography.labelBadge(
                color: ObsidianColors.primary,
              ),
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: ObsidianTypography.bodyMd(
                    color: ObsidianColors.onSurface,
                  ).copyWith(fontWeight: FontWeight.w600),
                ),
                const SizedBox(height: 4),
                Text(
                  description,
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
