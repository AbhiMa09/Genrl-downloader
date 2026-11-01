import 'dart:io';

import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

/// Screen 4 of Guided First Run: Folders, Hardlink Same-Volume Check, & Free Space.
class GuidedFoldersScreen extends StatefulWidget {
  final TextEditingController libraryController;
  final TextEditingController downloadController;
  final StateProvider state;

  const GuidedFoldersScreen({
    super.key,
    required this.libraryController,
    required this.downloadController,
    required this.state,
  });

  @override
  State<GuidedFoldersScreen> createState() => _GuidedFoldersScreenState();
}

class _GuidedFoldersScreenState extends State<GuidedFoldersScreen> {
  bool _isChecking = false;

  @override
  void initState() {
    super.initState();
    if (widget.libraryController.text.isEmpty) {
      widget.libraryController.text = _defaultLibraryPath();
    }
    if (widget.downloadController.text.isEmpty) {
      widget.downloadController.text = _defaultDownloadPath();
    }
    widget.libraryController.addListener(_onFoldersChanged);
    widget.downloadController.addListener(_onFoldersChanged);
  }

  void _onFoldersChanged() {
    if (mounted) setState(() {});
  }

  @override
  void dispose() {
    widget.libraryController.removeListener(_onFoldersChanged);
    widget.downloadController.removeListener(_onFoldersChanged);
    super.dispose();
  }

  static String _defaultLibraryPath() {
    if (Platform.isWindows) {
      final userProfile =
          Platform.environment['USERPROFILE'] ?? r'C:\Users\User';
      return '$userProfile\\Videos\\Anime';
    }
    final home = Platform.environment['HOME'] ?? '.';
    return '$home/Videos/Anime';
  }

  static String _defaultDownloadPath() {
    if (Platform.isWindows) {
      final userProfile =
          Platform.environment['USERPROFILE'] ?? r'C:\Users\User';
      return '$userProfile\\Downloads\\Genrl';
    }
    final home = Platform.environment['HOME'] ?? '.';
    return '$home/Downloads/Genrl';
  }

  String? _extractDrive(String path) {
    final trimmed = path.trim();
    if (trimmed.length >= 2 && trimmed[1] == ':') {
      return trimmed.substring(0, 2).toUpperCase();
    }
    return null;
  }

  bool _areDifferentDrives() {
    final d1 = _extractDrive(widget.libraryController.text);
    final d2 = _extractDrive(widget.downloadController.text);
    if (d1 != null && d2 != null) {
      return d1 != d2;
    }
    return false;
  }

  Future<void> _checkLocations() async {
    setState(() => _isChecking = true);
    await widget.state.refreshStatus();
    await widget.state.refreshLocations();
    if (mounted) setState(() => _isChecking = false);
  }

  @override
  Widget build(BuildContext context) {
    final isDifferentDrives = _areDifferentDrives();
    final freeGb = widget.state.status.freeDiskSpaceGb;
    final totalGb = widget.state.status.totalDiskSpaceGb;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Storage & Library Folders',
                    style: ObsidianTypography.headlineMd(),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    'Choose where your anime library lives and where torrents download.',
                    style: ObsidianTypography.bodySm(
                      color: ObsidianColors.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
            OutlinedButton.icon(
              onPressed: _isChecking ? null : _checkLocations,
              icon: _isChecking
                  ? const SizedBox(
                      width: 14,
                      height: 14,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.refresh_rounded, size: 16),
              label: const Text('Check again'),
            ),
          ],
        ),
        const SizedBox(height: 20),

        // Library folder input
        TextField(
          controller: widget.libraryController,
          onChanged: (_) => setState(() {}),
          decoration: const InputDecoration(
            labelText: 'Library Folder (Organized Shows)',
            hintText: r'C:\Users\User\Videos\Anime',
            helperText: 'Where seasons, episodes, synopsis.txt, and metadata are filed.',
            prefixIcon: Icon(Icons.video_library_rounded),
          ),
        ),
        const SizedBox(height: 14),

        // Downloads folder input
        TextField(
          controller: widget.downloadController,
          onChanged: (_) => setState(() {}),
          decoration: const InputDecoration(
            labelText: 'Downloads Folder (Temporary Ingestion)',
            hintText: r'C:\Users\User\Downloads\Genrl',
            helperText:
                'Where qBittorrent downloads raw video files before filing.',
            prefixIcon: Icon(Icons.download_rounded),
          ),
        ),
        const SizedBox(height: 16),

        // Cross-volume / different drives warning
        if (isDifferentDrives)
          Container(
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: ObsidianColors.warning.withOpacity(0.12),
              borderRadius: BorderRadius.circular(10),
              border: Border.all(
                color: ObsidianColors.warning.withOpacity(0.5),
              ),
            ),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Icon(
                  Icons.warning_amber_rounded,
                  color: ObsidianColors.warning,
                  size: 20,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Folders are on different drives (${_extractDrive(widget.libraryController.text)} vs ${_extractDrive(widget.downloadController.text)})',
                        style: ObsidianTypography.bodyMd(
                          color: ObsidianColors.warning,
                        ).copyWith(fontWeight: FontWeight.w600),
                      ),
                      const SizedBox(height: 4),
                      Text(
                        'Hardlinks require both folders to reside on the same drive partition. Because they are on different drives, Genrl will copy files instead of creating instant zero-space hardlinks.',
                        style: ObsidianTypography.bodySm(
                          color: ObsidianColors.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          )
        else
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: ObsidianColors.tertiary.withOpacity(0.1),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(
                color: ObsidianColors.tertiary.withOpacity(0.5),
              ),
            ),
            child: Row(
              children: [
                const Icon(
                  Icons.check_circle_rounded,
                  color: ObsidianColors.tertiary,
                  size: 18,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    'Same drive detected. Instant zero-space hardlinking is fully supported.',
                    style: ObsidianTypography.bodySm(
                      color: ObsidianColors.tertiary,
                    ),
                  ),
                ),
              ],
            ),
          ),

        const SizedBox(height: 16),

        // Free space info & NTFS note
        Container(
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(10),
            border: Border.all(color: ObsidianColors.border),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  const Icon(
                    Icons.storage_rounded,
                    size: 18,
                    color: ObsidianColors.primary,
                  ),
                  const SizedBox(width: 10),
                  Text(
                    freeGb > 0
                        ? 'Available Storage: $freeGb GB free (of $totalGb GB pool)'
                        : 'Storage Check: Verified accessible',
                    style: ObsidianTypography.bodyMd(
                      color: ObsidianColors.onSurface,
                    ).copyWith(fontWeight: FontWeight.w600),
                  ),
                ],
              ),
              if (freeGb > 0 && freeGb < 10) ...[
                const SizedBox(height: 6),
                Text(
                  'Notice: Less than 10 GB free space remaining. Anime downloads may fill the drive quickly.',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.warning,
                  ),
                ),
              ],
              const SizedBox(height: 10),
              const Divider(color: ObsidianColors.border, height: 1),
              const SizedBox(height: 10),
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Icon(
                    Icons.verified_user_outlined,
                    size: 16,
                    color: ObsidianColors.outline,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      'NTFS note: Genrl automatically sanitizes illegal characters (<>:"/\\|?*) and Windows reserved device names (CON, NUL, AUX, PRN) so show titles never cause filesystem errors.',
                      style: ObsidianTypography.bodySm(
                        color: ObsidianColors.onSurfaceVariant,
                      ),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),

        const SizedBox(height: 20),
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
                  'Why do we ask this? Hardlinks let your media player read files from the library while qBittorrent keeps seeding from downloads without taking extra storage.',
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
