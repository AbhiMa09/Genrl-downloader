import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class StepRootPath extends StatelessWidget {
  final TextEditingController pathController;
  final StateProvider state;

  const StepRootPath({
    super.key,
    required this.pathController,
    required this.state,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Target Library Destination',
          style: ObsidianTypography.headlineMd(),
        ),
        const SizedBox(height: 6),
        Text(
          'Select the root storage folder where your anime collection is organized and indexed.',
          style: ObsidianTypography.bodySm(
            color: ObsidianColors.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 20),
        TextField(
          controller: pathController,
          decoration: const InputDecoration(
            labelText: 'Library Root Path',
            hintText: '/mnt/anime/TV',
            prefixIcon: Icon(Icons.folder_outlined),
          ),
        ),
        const SizedBox(height: 16),
        Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: ObsidianColors.border),
          ),
          child: Row(
            children: [
              const Icon(
                Icons.storage_rounded,
                size: 18,
                color: ObsidianColors.tertiary,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  'Available capacity: ${state.status.freeDiskSpaceGb} GB free (of ${state.status.totalDiskSpaceGb} GB pool)',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurface,
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
