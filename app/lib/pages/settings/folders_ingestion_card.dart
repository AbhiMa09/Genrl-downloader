import 'package:flutter/material.dart';

import '../../theme/colors.dart';
import '../../theme/typography.dart';

/// Card for selecting filesystem ingestion strategy (hardlink/copy/move)
/// and series poster folder icon integration method.
class FoldersIngestionCard extends StatefulWidget {
  final String ingestionStrategy;
  final ValueChanged<String> onIngestionStrategyChanged;

  const FoldersIngestionCard({
    super.key,
    required this.ingestionStrategy,
    required this.onIngestionStrategyChanged,
  });

  @override
  State<FoldersIngestionCard> createState() => _FoldersIngestionCardState();
}

class _FoldersIngestionCardState extends State<FoldersIngestionCard> {
  String _folderIconMethod = 'Auto (Detect Desktop Env: KDE Plasma)';

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Ingestion Strategy
        Expanded(
          child: Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainer,
              borderRadius: BorderRadius.circular(10),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Ingestion Link Strategy',
                  style: ObsidianTypography.titleSm(),
                ),
                const SizedBox(height: 8),
                Material(
                  type: MaterialType.transparency,
                  child: Column(
                    children: [
                      RadioListTile<String>(
                        value: 'hardlink',
                        groupValue: widget.ingestionStrategy,
                        onChanged: (v) {
                          if (v != null) {
                            widget.onIngestionStrategyChanged(v);
                          }
                        },
                        title: Row(
                          children: [
                            const Text('Hardlink'),
                            const SizedBox(width: 6),
                            Container(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 4,
                                vertical: 1,
                              ),
                              decoration: BoxDecoration(
                                color: ObsidianColors.tertiary.withOpacity(0.2),
                                borderRadius: BorderRadius.circular(4),
                              ),
                              child: Text(
                                'Recommended',
                                style: ObsidianTypography.labelBadge(
                                  color: ObsidianColors.tertiary,
                                ),
                              ),
                            ),
                          ],
                        ),
                        subtitle: Text(
                          'Instant, zero disk space, preserves seed integrity',
                          style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline,
                          ),
                        ),
                        activeColor: ObsidianColors.primary,
                        contentPadding: EdgeInsets.zero,
                      ),
                      RadioListTile<String>(
                        value: 'copy',
                        groupValue: widget.ingestionStrategy,
                        onChanged: (v) {
                          if (v != null) {
                            widget.onIngestionStrategyChanged(v);
                          }
                        },
                        title: const Text('Copy (Duplicate file)'),
                        subtitle: Text(
                          'Duplicates raw bytes; high I/O consumption',
                          style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline,
                          ),
                        ),
                        activeColor: ObsidianColors.primary,
                        contentPadding: EdgeInsets.zero,
                      ),
                      RadioListTile<String>(
                        value: 'move',
                        groupValue: widget.ingestionStrategy,
                        onChanged: (v) {
                          if (v != null) {
                            widget.onIngestionStrategyChanged(v);
                          }
                        },
                        title: const Text('Move (Atomic filesystem move)'),
                        subtitle: Text(
                          'Terminates active seeding; removes from staging',
                          style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline,
                          ),
                        ),
                        activeColor: ObsidianColors.primary,
                        contentPadding: EdgeInsets.zero,
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(width: 12),
        // Folder Icon Method
        Expanded(
          child: Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainer,
              borderRadius: BorderRadius.circular(10),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Expanded(
                      child: Text(
                        'Series Poster Folder Icon Generator',
                        style: ObsidianTypography.titleSm(),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    const SizedBox(width: 8),
                    Text(
                      'Freedesktop.org',
                      style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.outline,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                Text(
                  'Inject poster metadata directly into directory browser view:',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 8),
                DropdownButtonFormField<String>(
                  value: _folderIconMethod,
                  isExpanded: true,
                  decoration: const InputDecoration(
                    border: OutlineInputBorder(),
                  ),
                  dropdownColor: ObsidianColors.surfaceContainerHigh,
                  items: const [
                    DropdownMenuItem(
                      value: 'Auto (Detect Desktop Env: KDE Plasma)',
                      child: Text(
                        'Auto (Detect Desktop Env: KDE Plasma)',
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    DropdownMenuItem(
                      value: 'KDE Plasma (KIO / directory.png)',
                      child: Text(
                        'KDE Plasma (KIO / directory.png)',
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    DropdownMenuItem(
                      value: 'GNOME (gvfs-metadata)',
                      child: Text(
                        'GNOME (gvfs-metadata)',
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    DropdownMenuItem(
                      value: 'Off (Disabled)',
                      child: Text(
                        'Off (Disabled)',
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ],
                  onChanged: (v) {
                    if (v != null) {
                      setState(() => _folderIconMethod = v);
                    }
                  },
                ),
                const SizedBox(height: 12),
                Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: Row(
                    children: [
                      const Icon(
                        Icons.palette_rounded,
                        size: 16,
                        color: ObsidianColors.primary,
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          'Plasma KIO hook configured: creates directory.png and sets desktop.ini',
                          style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }
}
