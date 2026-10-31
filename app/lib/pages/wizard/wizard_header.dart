import 'package:flutter/material.dart';

import '../../constants/app_version.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class WizardHeader extends StatelessWidget {
  final VoidCallback onClose;
  final int currentStep;

  const WizardHeader({super.key, required this.onClose, this.currentStep = 4});

  String get _title {
    switch (currentStep) {
      case 0:
        return 'MyAnimeList Account Connection';
      case 1:
        return 'qBittorrent Integration';
      case 2:
        return 'Media Library & Storage Paths';
      case 3:
        return 'Quality & Download Rules';
      case 4:
      default:
        return 'Initial Library Synchronization & Review';
    }
  }

  String get _subtitle {
    switch (currentStep) {
      case 0:
        return 'Connect your MyAnimeList account to sync your watching list and plan download queues.';
      case 1:
        return 'Connect to local or remote qBittorrent WebUI daemon for automated queue management.';
      case 2:
        return 'Configure base directories for ongoing torrent downloads and organized anime library folders.';
      case 3:
        return 'Configure resolution floors, release group preferences, and automation filters.';
      case 4:
      default:
        return 'Review detected library manifests, ingestion queue volume, and confirm automation daemon state before starting.';
    }
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(24),
      decoration: const BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 42,
            height: 42,
            decoration: BoxDecoration(
              color: ObsidianColors.secondary.withOpacity(0.2),
              borderRadius: BorderRadius.circular(10),
            ),
            child: const Icon(
              Icons.terminal_rounded,
              size: 24,
              color: ObsidianColors.primary,
            ),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text(
                      'GENRL DOWNLOADER SETUP WIZARD',
                      style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.primary,
                      ),
                    ),
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 5,
                        vertical: 1,
                      ),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHighest,
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        kAppVersion,
                        style: ObsidianTypography.labelBadge(
                          color: ObsidianColors.outline,
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 6,
                        vertical: 1.5,
                      ),
                      decoration: BoxDecoration(
                        color: ObsidianColors.secondary.withOpacity(0.2),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        'Step ${currentStep + 1} of 5',
                        style: ObsidianTypography.labelBadge(
                          color: ObsidianColors.secondary,
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 6),
                Text(_title, style: ObsidianTypography.headlineMd()),
                const SizedBox(height: 4),
                Text(
                  _subtitle,
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(
              Icons.close_rounded,
              color: ObsidianColors.outline,
            ),
            onPressed: onClose,
          ),
        ],
      ),
    );
  }
}
