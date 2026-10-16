import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'folders_ingestion_card.dart';

class SettingsFoldersSection extends StatefulWidget {
  final StateProvider state;
  final TextEditingController libraryFolderController;
  final TextEditingController downloadFolderController;
  final TextEditingController namingPatternController;
  final String ingestionStrategy;
  final ValueChanged<String> onIngestionStrategyChanged;

  const SettingsFoldersSection({
    super.key,
    required this.state,
    required this.libraryFolderController,
    required this.downloadFolderController,
    required this.namingPatternController,
    required this.ingestionStrategy,
    required this.onIngestionStrategyChanged,
  });

  @override
  State<SettingsFoldersSection> createState() => _SettingsFoldersSectionState();
}

class _SettingsFoldersSectionState extends State<SettingsFoldersSection> {
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
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Row(
                  children: [
                    Container(
                      padding: const EdgeInsets.all(6),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHigh,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: const Icon(
                        Icons.folder_copy_rounded,
                        size: 20,
                        color: ObsidianColors.primary,
                      ),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'Folders & Filesystem Ingestion',
                            style: ObsidianTypography.titleSm(),
                            overflow: TextOverflow.ellipsis,
                          ),
                          Text(
                            'Manage target directories, rename patterns, hardlinking rules, and desktop folder art',
                            style: ObsidianTypography.bodySm(
                              color: ObsidianColors.outline,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  'Btrfs / CoW Aware',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.onSurfaceVariant,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),

          // Anime Library Path
          _buildPathPicker(
            label: 'Anime Library Root Path',
            metric: '3.42 TB / 8.00 TB used (57% free)',
            icon: Icons.folder_special_rounded,
            controller: widget.libraryFolderController,
            state: widget.state,
          ),
          const SizedBox(height: 12),

          // Download Staging Path
          _buildPathPicker(
            label: 'Download Staging / Client Incomplete Ingestion',
            metric: 'NVMe Scratch Cache',
            icon: Icons.hourglass_top_rounded,
            controller: widget.downloadFolderController,
            state: widget.state,
          ),
          const SizedBox(height: 16),

          // Naming Pattern Configurator
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainer,
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: ObsidianColors.border),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(
                      'Standard Episode Naming Pattern',
                      style: ObsidianTypography.monoCode(),
                    ),
                    Text(
                      'Token Interpolation Active',
                      style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.primary,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 6),
                Container(
                  height: 38,
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: ObsidianColors.border),
                  ),
                  child: Row(
                    children: [
                      const Icon(
                        Icons.text_fields_rounded,
                        size: 16,
                        color: ObsidianColors.outline,
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: TextField(
                          controller: widget.namingPatternController,
                          style: ObsidianTypography.monoCode(
                            color: ObsidianColors.primary,
                          ),
                          decoration: const InputDecoration(
                            border: InputBorder.none,
                            isDense: true,
                            contentPadding: EdgeInsets.zero,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 8),

                // Helper Chips
                Wrap(
                  spacing: 6,
                  runSpacing: 4,
                  children: [
                    Text(
                      'Insert Tokens:',
                      style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.outline,
                      ),
                    ),
                    _buildTokenChip('{Series Title}'),
                    _buildTokenChip('{Season:02d}'),
                    _buildTokenChip('{Episode:02d}'),
                    _buildTokenChip('{Resolution}'),
                    _buildTokenChip('{Group}'),
                    _buildTokenChip('{CRC32}'),
                  ],
                ),
                const SizedBox(height: 10),

                // Live Preview Callout
                Container(
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLowest,
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Text(
                            'LIVE RENAMER PREVIEW',
                            style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.tertiary,
                            ),
                          ),
                          Text(
                            'Target Ext: .mkv',
                            style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.outline,
                            ),
                          ),
                        ],
                      ),
                      Builder(
                        builder: (context) {
                          final state = widget.state;
                          final title = state.shows.isNotEmpty
                              ? state.shows.first.title
                              : '{Series Title}';
                          return Text(
                            '/mnt/anime/TV/$title/Season 01/$title - S01E01 [1080p].mkv',
                            style: ObsidianTypography.monoCode(
                              color: ObsidianColors.onSurface,
                            ),
                          );
                        },
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),

          // Dual Column: Ingestion Link Strategy & Series Poster Folder Icon Method
          FoldersIngestionCard(
            ingestionStrategy: widget.ingestionStrategy,
            onIngestionStrategyChanged: widget.onIngestionStrategyChanged,
          ),
        ],
      ),
    );
  }

  Widget _buildPathPicker({
    required String label,
    required String metric,
    required IconData icon,
    required TextEditingController controller,
    required StateProvider state,
  }) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Expanded(
              child: Text(
                label,
                style: ObsidianTypography.monoCode(),
                overflow: TextOverflow.ellipsis,
              ),
            ),
            const SizedBox(width: 8),
            Text(
              metric,
              style: ObsidianTypography.monoCode(
                color: ObsidianColors.tertiary,
              ),
            ),
          ],
        ),
        const SizedBox(height: 4),
        Row(
          children: [
            Expanded(
              child: Container(
                height: 38,
                padding: const EdgeInsets.symmetric(horizontal: 10),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Row(
                  children: [
                    Icon(icon, size: 16, color: ObsidianColors.primary),
                    const SizedBox(width: 8),
                    Expanded(
                      child: TextField(
                        controller: controller,
                        style: ObsidianTypography.monoCode(),
                        decoration: const InputDecoration(
                          border: InputBorder.none,
                          isDense: true,
                          contentPadding: EdgeInsets.zero,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(width: 8),
            ElevatedButton.icon(
              onPressed: () {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: Text('Selecting path for ${controller.text}'),
                  ),
                );
              },
              icon: const Icon(Icons.drive_file_move_rounded, size: 16),
              label: const Text('Browse...'),
              style: ElevatedButton.styleFrom(
                backgroundColor: ObsidianColors.surfaceContainerHigh,
                foregroundColor: ObsidianColors.onSurface,
                elevation: 0,
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildTokenChip(String token) {
    return InkWell(
      onTap: () {
        widget.namingPatternController.text += token;
      },
      borderRadius: BorderRadius.circular(4),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainerHigh,
          borderRadius: BorderRadius.circular(4),
        ),
        child: Text(
          '+ $token',
          style: ObsidianTypography.labelBadge(color: ObsidianColors.primary),
        ),
      ),
    );
  }
}
