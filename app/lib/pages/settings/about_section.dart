import 'package:flutter/material.dart';

import '../../constants/app_version.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class SettingsAboutSection extends StatelessWidget {
  const SettingsAboutSection({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                padding: const EdgeInsets.all(6),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: const Icon(
                  Icons.info_outline_rounded,
                  size: 20,
                  color: ObsidianColors.primary,
                ),
              ),
              const SizedBox(width: 10),
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'About Genrl Downloader',
                    style: ObsidianTypography.titleSm(),
                  ),
                  Text(
                    'Software release, build details and platform environment',
                    style: ObsidianTypography.bodySm(
                      color: ObsidianColors.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 16),
          const Divider(height: 1, color: ObsidianColors.border),
          const SizedBox(height: 16),
          Wrap(
            spacing: 24,
            runSpacing: 12,
            children: [
              _buildDetailItem('Version', kAppVersion),
              _buildDetailItem('Git Short Hash', kGitShortHash),
              _buildDetailItem('Build Date', kBuildDate),
              _buildDetailItem('Full Build', kAppBuildInfo),
              _buildDetailItem('Platform', 'Linux x86_64'),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildDetailItem(String label, String value) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label.toUpperCase(),
          style: ObsidianTypography.labelBadge(color: ObsidianColors.outline),
        ),
        const SizedBox(height: 4),
        Text(
          value,
          style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
        ),
      ],
    );
  }
}
