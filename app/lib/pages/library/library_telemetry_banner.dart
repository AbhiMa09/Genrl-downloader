import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class LibraryTelemetryBanner extends StatelessWidget {
  final int seriesCount;
  final String totalTb;
  final String libraryPath;
  final StateProvider state;

  const LibraryTelemetryBanner({
    super.key,
    required this.seriesCount,
    required this.totalTb,
    required this.libraryPath,
    required this.state,
  });

  @override
  Widget build(BuildContext context) {
    final qbLabel = state.status.qbitConnected
        ? 'qB: ${state.status.qbitVersion}'
        : 'qB: Offline';
    final fsLabel =
        'FS: Btrfs (Hardlink: ${state.sameFilesystem ? "Supported" : "Cross-FS"})';

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 8),
      decoration: const BoxDecoration(
        color: ObsidianColors.surfaceContainerLowest,
        border: Border(bottom: BorderSide(color: ObsidianColors.border)),
      ),
      child: Wrap(
        alignment: WrapAlignment.spaceBetween,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: 16,
        runSpacing: 8,
        children: [
          Wrap(
            crossAxisAlignment: WrapCrossAlignment.center,
            spacing: 14,
            runSpacing: 6,
            children: [
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    'Anime Library',
                    style: ObsidianTypography.headlineMd(fontSize: 16),
                  ),
                  const SizedBox(width: 12),
                  Container(width: 1, height: 14, color: ObsidianColors.border),
                  const SizedBox(width: 12),
                  const Icon(
                    Icons.folder_copy_rounded,
                    size: 16,
                    color: ObsidianColors.primary,
                  ),
                  const SizedBox(width: 6),
                  Text(
                    '$seriesCount Series',
                    style: ObsidianTypography.monoMetric(
                      color: ObsidianColors.onSurface,
                      fontWeight: FontWeight.bold,
                      fontSize: 12,
                    ),
                  ),
                  Text(
                    ' / ',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.outline,
                    ),
                  ),
                  Text(
                    '$totalTb TB on $libraryPath',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.onSurfaceVariant,
                      fontSize: 11,
                    ),
                  ),
                ],
              ),
              Container(width: 1, height: 12, color: ObsidianColors.border),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Container(
                    width: 6,
                    height: 6,
                    decoration: const BoxDecoration(
                      color: ObsidianColors.tertiary,
                      shape: BoxShape.circle,
                    ),
                  ),
                  const SizedBox(width: 6),
                  Text(
                    'All Folders Monitored',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.onSurfaceVariant,
                      fontSize: 11,
                    ),
                  ),
                ],
              ),
              Container(width: 1, height: 12, color: ObsidianColors.border),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(
                    Icons.sync_rounded,
                    size: 14,
                    color: ObsidianColors.tertiary,
                  ),
                  const SizedBox(width: 6),
                  Text(
                    'Library Ingestion Ready',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.outline,
                      fontSize: 11,
                    ),
                  ),
                ],
              ),
            ],
          ),
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  fsLabel,
                  style: ObsidianTypography.monoCode(
                    color: ObsidianColors.outline,
                    fontSize: 10,
                  ),
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
                  qbLabel,
                  style: ObsidianTypography.monoCode(
                    color: ObsidianColors.tertiary,
                    fontSize: 10,
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
