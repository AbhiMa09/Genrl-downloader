import 'package:flutter/material.dart';

import '../../theme/colors.dart';
import '../../theme/typography.dart';

/// Screen 1 of Guided First Run: Welcome & Prerequisites checklist.
class GuidedWelcomeScreen extends StatelessWidget {
  final VoidCallback onNext;

  const GuidedWelcomeScreen({super.key, required this.onNext});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Container(
              padding: const EdgeInsets.all(10),
              decoration: BoxDecoration(
                color: ObsidianColors.primary.withOpacity(0.15),
                borderRadius: BorderRadius.circular(10),
              ),
              child: const Icon(
                Icons.auto_awesome_rounded,
                color: ObsidianColors.primary,
                size: 24,
              ),
            ),
            const SizedBox(width: 14),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Welcome to Genrl Downloader',
                    style: ObsidianTypography.headlineMd(),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    'Automated anime watching tracker and library manager for Windows.',
                    style: ObsidianTypography.bodySm(
                      color: ObsidianColors.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        const SizedBox(height: 24),
        Container(
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(color: ObsidianColors.border),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'What Genrl Does:',
                style: ObsidianTypography.titleSm(
                  color: ObsidianColors.onSurface,
                ),
              ),
              const SizedBox(height: 8),
              Text(
                'Genrl connects to your MyAnimeList account to find anime you are watching, searches release feeds for the best quality releases, and downloads episodes automatically via qBittorrent.\nIt files everything into organized folders with metadata, zero double-storage via hardlinks, and no manual dragging required.',
                style: ObsidianTypography.bodyMd(
                  color: ObsidianColors.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 20),
        Text(
          'What You Will Need:',
          style: ObsidianTypography.titleSm(color: ObsidianColors.onSurface),
        ),
        const SizedBox(height: 12),
        const _PrereqTile(
          icon: Icons.account_circle_outlined,
          title: 'MyAnimeList Account',
          subtitle:
              'To sync your Watching and Plan to Watch lists automatically.',
        ),
        const SizedBox(height: 10),
        const _PrereqTile(
          icon: Icons.downloading_rounded,
          title: 'qBittorrent (Installed & Running)',
          subtitle: 'The open-source torrent client Genrl talks to over local Web UI.',
        ),
        const SizedBox(height: 10),
        const _PrereqTile(
          icon: Icons.disc_full_outlined,
          title: 'Free Disk Space (Recommended >= 10 GB)',
          subtitle: 'A local drive partition where your downloads and anime library live.',
        ),
        const SizedBox(height: 24),
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainerHighest.withOpacity(0.5),
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: ObsidianColors.border.withOpacity(0.5)),
          ),
          child: Row(
            children: [
              const Icon(
                Icons.info_outline_rounded,
                size: 16,
                color: ObsidianColors.primary,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  'Why do we ask this? Genrl coordinates your existing tools without replacing them, ensuring your files and accounts remain in your full control.',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurfaceVariant,
                  ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _PrereqTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;

  const _PrereqTile({
    required this.icon,
    required this.title,
    required this.subtitle,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Row(
        children: [
          Icon(icon, size: 20, color: ObsidianColors.tertiary),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurface,
                  ).copyWith(fontWeight: FontWeight.w600),
                ),
                Text(
                  subtitle,
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.outline,
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
