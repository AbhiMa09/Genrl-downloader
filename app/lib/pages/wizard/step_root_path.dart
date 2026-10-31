import 'dart:io';

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
    return ListenableBuilder(
      listenable: pathController,
      builder: (context, _) {
        final path = pathController.text.trim();
        final bool folderExists =
            path.isNotEmpty && Directory(path).existsSync();

        Directory existingDir = Directory(path.isNotEmpty ? path : '/tmp');
        while (!existingDir.existsSync() &&
            existingDir.parent.path != existingDir.path) {
          existingDir = existingDir.parent;
        }

        int freeGb = state.status.freeDiskSpaceGb;
        int totalGb = state.status.totalDiskSpaceGb;

        if (freeGb <= 0 && existingDir.existsSync()) {
          try {
            final res = Process.runSync('df', ['-k', existingDir.path]);
            if (res.exitCode == 0) {
              final lines = (res.stdout as String).trim().split('\n');
              if (lines.length >= 2) {
                final parts = lines[1].split(RegExp(r'\s+'));
                if (parts.length >= 4) {
                  final totalK = int.tryParse(parts[1]) ?? 0;
                  final availK = int.tryParse(parts[3]) ?? 0;
                  totalGb = (totalK / (1024 * 1024)).round();
                  freeGb = (availK / (1024 * 1024)).round();
                }
              }
            }
          } catch (_) {}
        }

        final capacityText = freeGb > 0
            ? 'Available capacity: $freeGb GB free (of $totalGb GB pool)'
            : 'Storage pool accessible';
        final statusNote = !folderExists && path.isNotEmpty
            ? ' — folder will be created upon confirmation'
            : '';

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
                hintText: 'e.g. /home/user/anime',
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
                      '$capacityText$statusNote',
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
      },
    );
  }
}
