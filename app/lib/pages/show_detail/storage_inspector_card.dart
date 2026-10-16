import 'package:flutter/material.dart';

import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'dialogs.dart';
import 'show_detail_utils.dart';

class StorageInspectorCard extends StatelessWidget {
  final StateProvider state;
  final LibraryShow show;
  final String onDiskString;

  const StorageInspectorCard({
    super.key,
    required this.state,
    required this.show,
    required this.onDiskString,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Row(
                children: [
                  const Icon(
                    Icons.folder_outlined,
                    size: 16,
                    color: ObsidianColors.tertiary,
                  ),
                  const SizedBox(width: 6),
                  Text(
                    'ROOT DIRECTORY',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ],
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  'EXT4',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.tertiary,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),

          // Path display box
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: SelectableText(
              show.folderPath ?? '/mnt/anime/${show.title}/',
              style: ObsidianTypography.monoCode(
                color: ObsidianColors.onSurfaceVariant,
              ).copyWith(fontSize: 11),
            ),
          ),
          const SizedBox(height: 10),

          // On-Disk Footprint
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Flexible(
                child: Text(
                  'On-Disk Footprint:',
                  style: ObsidianTypography.monoCode(
                    color: ObsidianColors.outline,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 8),
              Text(
                onDiskString,
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.primary,
                ).copyWith(fontWeight: FontWeight.bold),
              ),
            ],
          ),
          const SizedBox(height: 12),

          // Open in File Manager Action
          SizedBox(
            width: double.infinity,
            child: FilledButton.icon(
              onPressed: () => openFileOrFolder(show.folderPath),
              icon: const Icon(
                Icons.folder_open_rounded,
                size: 16,
                color: ObsidianColors.primary,
              ),
              label: const Text('Open in File Manager'),
              style: FilledButton.styleFrom(
                backgroundColor: ObsidianColors.surfaceContainerHigh,
                foregroundColor: ObsidianColors.onSurface,
                padding: const EdgeInsets.symmetric(vertical: 10),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(8),
                ),
              ),
            ),
          ),
          const SizedBox(height: 8),

          // Manual Search & Change Match row
          Row(
            children: [
              Expanded(
                child: OutlinedButton.icon(
                  onPressed: () {
                    state.setSearchFilter(show.title);
                    state.selectPage(4); // Search tab
                  },
                  icon: const Icon(Icons.search, size: 14),
                  label: const Text('Manual Search'),
                  style: OutlinedButton.styleFrom(
                    foregroundColor: ObsidianColors.onSurfaceVariant,
                    side: const BorderSide(color: ObsidianColors.border),
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: OutlinedButton.icon(
                  onPressed: () {
                    showChangeMatchDialog(context, state, show);
                  },
                  icon: const Icon(Icons.swap_horiz, size: 14),
                  label: const Text('Change Match'),
                  style: OutlinedButton.styleFrom(
                    foregroundColor: ObsidianColors.onSurfaceVariant,
                    side: const BorderSide(color: ObsidianColors.border),
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
