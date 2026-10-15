import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class WizardHeader extends StatelessWidget {
  final VoidCallback onClose;

  const WizardHeader({super.key, required this.onClose});

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
            child: const Icon(Icons.terminal_rounded,
                size: 24, color: ObsidianColors.primary),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text('GENRL DOWNLOADER SETUP WIZARD',
                        style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.primary)),
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(
                          horizontal: 5, vertical: 1),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHighest,
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text('v2.4.1',
                          style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.outline)),
                    ),
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(
                          horizontal: 6, vertical: 1.5),
                      decoration: BoxDecoration(
                        color: ObsidianColors.secondary.withOpacity(0.2),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text('Step 5 of 5',
                          style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.secondary)),
                    ),
                  ],
                ),
                const SizedBox(height: 6),
                Text('Initial Library Synchronization & Review',
                    style: ObsidianTypography.headlineMd()),
                const SizedBox(height: 4),
                Text(
                  'Review detected library manifests, ingestion queue volume, and confirm automation daemon state before starting.',
                  style: ObsidianTypography.bodySm(
                      color: ObsidianColors.onSurfaceVariant),
                ),
              ],
            ),
          ),
          IconButton(
            icon:
                const Icon(Icons.close_rounded, color: ObsidianColors.outline),
            onPressed: onClose,
          ),
        ],
      ),
    );
  }
}
