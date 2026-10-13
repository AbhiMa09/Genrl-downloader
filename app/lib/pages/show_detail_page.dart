import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/library_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/anime_poster.dart';

class ShowDetailPage extends StatefulWidget {
  final LibraryShow show;

  const ShowDetailPage({
    super.key,
    required this.show,
  });

  @override
  State<ShowDetailPage> createState() => _ShowDetailPageState();
}

class _ShowDetailPageState extends State<ShowDetailPage> {
  late TextEditingController _folderController;
  bool _isEditingFolder = false;
  bool _isChecking = false;
  Map<String, dynamic>? _checkResult;

  @override
  void initState() {
    super.initState();
    _folderController = TextEditingController(text: widget.show.folderPath ?? '');
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<StateProvider>().loadShowDetail(widget.show.malId);
    });
  }

  @override
  void didUpdateWidget(covariant ShowDetailPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.show.folderPath != widget.show.folderPath) {
      _folderController.text = widget.show.folderPath ?? '';
    }
  }

  @override
  void dispose() {
    _folderController.dispose();
    super.dispose();
  }

  Future<void> _runCheck(String path) async {
    if (path.trim().isEmpty) {
      setState(() => _checkResult = null);
      return;
    }
    setState(() => _isChecking = true);
    final state = context.read<StateProvider>();
    final res = await state.checkLocationLive(path.trim());
    if (mounted) {
      setState(() {
        _isChecking = false;
        _checkResult = res;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();
    final show = state.selectedShow ?? widget.show;

    return Material(
      color: Colors.transparent,
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Breadcrumb Navigation & Top Action Bar
            Row(
              children: [
                OutlinedButton.icon(
                  onPressed: () => state.closeShowDetail(),
                  icon: const Icon(Icons.arrow_back_rounded, size: 16),
                  label: const Text('Back to Library'),
                  style: OutlinedButton.styleFrom(
                    foregroundColor: ObsidianColors.onSurface,
                    side: const BorderSide(color: ObsidianColors.border),
                    padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                  ),
                ),
                const Spacer(),
                Tooltip(
                  message: 'Download Season',
                  child: FilledButton.icon(
                    onPressed: () => state.downloadSeason(show.malId),
                    icon: const Icon(Icons.download_rounded, size: 16),
                    label: const Text('Download Season'),
                    style: FilledButton.styleFrom(
                      backgroundColor: ObsidianColors.tertiary,
                      foregroundColor: Colors.black,
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                Tooltip(
                  message: 'Configure Per-Show Rules',
                  child: OutlinedButton.icon(
                    onPressed: () {
                      state.closeShowDetail();
                      state.selectPage(6); // Go to Rules
                    },
                    icon: const Icon(Icons.tune_rounded, size: 16, color: ObsidianColors.primary),
                    label: Text('Per-Show Rules', style: ObsidianTypography.bodySm(color: ObsidianColors.primary)),
                    style: OutlinedButton.styleFrom(
                      side: const BorderSide(color: ObsidianColors.border),
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                    ),
                  ),
                ),
              ],
            ),
          const SizedBox(height: 20),
            // Header Hero Section (Poster + Metadata)
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // 2:3 Poster
                SizedBox(
                  width: 160,
                  height: 240,
                  child: AnimePoster(
                    urlOrPath: show.posterUrl,
                    aspectRatio: 2 / 3,
                    borderRadius: BorderRadius.circular(10),
                  ),
                ),
                const SizedBox(width: 20),

                // Metadata Details
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(show.title, style: ObsidianTypography.headlineLg()),
                      if (show.englishTitle != null) ...[
                        const SizedBox(height: 4),
                        Text(
                          show.englishTitle!,
                          style: ObsidianTypography.bodyMd(
                              color: ObsidianColors.onSurfaceVariant),
                        ),
                      ],
                      const SizedBox(height: 12),

                      // Metric badges
                      Row(
                        children: [
                          _MetaBadge(
                            label: '★ ${show.score}',
                            color: ObsidianColors.primary,
                          ),
                          const SizedBox(width: 8),
                          _MetaBadge(
                            label: '${show.startYear}',
                            color: ObsidianColors.outline,
                          ),
                          const SizedBox(width: 8),
                          _MetaBadge(
                            label: '${show.totalEpisodes} Episodes',
                            color: ObsidianColors.tertiary,
                          ),
                          const SizedBox(width: 8),
                          _MetaBadge(
                            label: show.airingStatus,
                            color: ObsidianColors.success,
                          ),
                        ],
                      ),
                      const SizedBox(height: 12),

                      // Genres
                      Wrap(
                        spacing: 6,
                        runSpacing: 6,
                        children: show.genres.map((g) {
                          return Container(
                            padding: const EdgeInsets.symmetric(
                                horizontal: 8, vertical: 3),
                            decoration: BoxDecoration(
                              color: ObsidianColors.surfaceContainerHigh,
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text(
                              g,
                              style: ObsidianTypography.labelBadge(
                                  color: ObsidianColors.onSurfaceVariant),
                            ),
                          );
                        }).toList(),
                      ),
                      const SizedBox(height: 14),

                      // Folder Path Section (Default vs Override)
                      Container(
                        padding: const EdgeInsets.all(12),
                        decoration: BoxDecoration(
                          color: ObsidianColors.surfaceContainerLowest,
                          borderRadius: BorderRadius.circular(8),
                          border: Border.all(
                            color: show.folderPath != null && show.folderPath!.isNotEmpty
                                ? ObsidianColors.primary.withOpacity(0.3)
                                : ObsidianColors.border,
                            width: 0.8,
                          ),
                        ),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Row(
                              children: [
                                const Icon(Icons.folder_open_rounded,
                                    size: 16, color: ObsidianColors.primary),
                                const SizedBox(width: 8),
                                Text(
                                  show.folderPath != null && show.folderPath!.isNotEmpty
                                      ? 'Custom Storage Folder (Override)'
                                      : 'Library Storage Location',
                                  style: ObsidianTypography.labelBadge(
                                      color: ObsidianColors.onSurface),
                                ),
                                const Spacer(),
                                if (!_isEditingFolder)
                                  TextButton.icon(
                                    style: TextButton.styleFrom(
                                      padding: const EdgeInsets.symmetric(
                                          horizontal: 8, vertical: 4),
                                      minimumSize: Size.zero,
                                      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                                    ),
                                    icon: const Icon(Icons.edit_rounded,
                                        size: 14, color: ObsidianColors.primary),
                                    label: Text(
                                      show.folderPath != null && show.folderPath!.isNotEmpty
                                          ? 'Change'
                                          : 'Override',
                                      style: ObsidianTypography.labelBadge(
                                          color: ObsidianColors.primary),
                                    ),
                                    onPressed: () {
                                      setState(() {
                                        _isEditingFolder = true;
                                        _folderController.text = show.folderPath ?? '';
                                      });
                                      if ((show.folderPath ?? '').isNotEmpty) {
                                        _runCheck(show.folderPath!);
                                      }
                                    },
                                  ),
                              ],
                            ),
                            const SizedBox(height: 6),
                            if (!_isEditingFolder) ...[
                              Text(
                                show.folderPath != null && show.folderPath!.isNotEmpty
                                    ? show.folderPath!
                                    : 'Default library location (determined by show title & year)',
                                style: ObsidianTypography.monoCode(
                                  color: show.folderPath != null && show.folderPath!.isNotEmpty
                                      ? ObsidianColors.onSurface
                                      : ObsidianColors.outline,
                                ),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ] else ...[
                              TextField(
                                controller: _folderController,
                                style: ObsidianTypography.monoCode(),
                                onChanged: (val) => _runCheck(val),
                                decoration: InputDecoration(
                                  hintText: '/path/to/custom/show/directory',
                                  hintStyle: ObsidianTypography.monoCode(
                                      color: ObsidianColors.outline),
                                  isDense: true,
                                  filled: true,
                                  fillColor: ObsidianColors.surfaceContainerHigh,
                                  contentPadding: const EdgeInsets.symmetric(
                                      horizontal: 10, vertical: 8),
                                  border: OutlineInputBorder(
                                    borderRadius: BorderRadius.circular(6),
                                    borderSide:
                                        BorderSide(color: ObsidianColors.border),
                                  ),
                                ),
                              ),
                              const SizedBox(height: 8),
                              // Live validation badge
                              if (_isChecking)
                                const Row(
                                  children: [
                                    SizedBox(
                                      width: 12,
                                      height: 12,
                                      child: CircularProgressIndicator(
                                        strokeWidth: 2,
                                        color: ObsidianColors.primary,
                                      ),
                                    ),
                                    SizedBox(width: 6),
                                    Text('Checking directory...'),
                                  ],
                                )
                              else if (_checkResult != null) ...[
                                Row(
                                  children: [
                                    Icon(
                                      _checkResult!['is_valid'] == true
                                          ? Icons.check_circle_rounded
                                          : Icons.error_outline_rounded,
                                      size: 14,
                                      color: _checkResult!['is_valid'] == true
                                          ? ObsidianColors.success
                                          : ObsidianColors.error,
                                    ),
                                    const SizedBox(width: 6),
                                    Text(
                                      _checkResult!['is_valid'] == true
                                          ? 'Writable • ${_checkResult!['free_space_human'] ?? 'Unknown'} free'
                                          : (_checkResult!['error'] ?? 'Invalid path'),
                                      style: ObsidianTypography.labelBadge(
                                        color: _checkResult!['is_valid'] == true
                                            ? ObsidianColors.success
                                            : ObsidianColors.error,
                                      ),
                                    ),
                                  ],
                                ),
                              ],
                              const SizedBox(height: 8),
                              Row(
                                mainAxisAlignment: MainAxisAlignment.end,
                                children: [
                                  if (show.folderPath != null && show.folderPath!.isNotEmpty)
                                    TextButton(
                                      onPressed: () async {
                                        await state.saveShowFolder(show.malId, '');
                                        if (mounted) {
                                          setState(() {
                                            _isEditingFolder = false;
                                            _folderController.text = '';
                                            _checkResult = null;
                                          });
                                        }
                                      },
                                      child: Text(
                                        'Reset to Default',
                                        style: ObsidianTypography.labelBadge(
                                            color: ObsidianColors.error),
                                      ),
                                    ),
                                  const SizedBox(width: 8),
                                  TextButton(
                                    onPressed: () {
                                      setState(() {
                                        _isEditingFolder = false;
                                        _folderController.text = show.folderPath ?? '';
                                        _checkResult = null;
                                      });
                                    },
                                    child: Text(
                                      'Cancel',
                                      style: ObsidianTypography.labelBadge(
                                          color: ObsidianColors.outline),
                                    ),
                                  ),
                                  const SizedBox(width: 8),
                                  FilledButton(
                                    style: FilledButton.styleFrom(
                                      backgroundColor: ObsidianColors.primary,
                                      padding: const EdgeInsets.symmetric(
                                          horizontal: 14, vertical: 6),
                                    ),
                                    onPressed: () async {
                                      final path = _folderController.text.trim();
                                      await state.saveShowFolder(show.malId, path);
                                      if (mounted) {
                                        setState(() {
                                          _isEditingFolder = false;
                                          _checkResult = null;
                                        });
                                      }
                                    },
                                    child: Text(
                                      'Save',
                                      style: ObsidianTypography.labelBadge(
                                          color: Colors.black),
                                    ),
                                  ),
                                ],
                              ),
                            ],
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 24),

            // Synopsis narrative block
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLow,
                borderRadius: BorderRadius.circular(10),
                border: Border.all(color: ObsidianColors.border),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('SYNOPSIS', style: ObsidianTypography.titleSm()),
                  const SizedBox(height: 8),
                  Text(
                    show.synopsis,
                    style: ObsidianTypography.bodyMd(),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 24),

            // Episodes Grid Matrix
            Text('EPISODES MATRIX', style: ObsidianTypography.titleSm()),
            const SizedBox(height: 12),

            if (show.totalEpisodes == 0 && show.episodes.isEmpty)
              Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLowest,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Row(
                  children: [
                    const Icon(Icons.info_outline_rounded, size: 18, color: ObsidianColors.outline),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Text(
                        'Total episode count unknown or currently airing. Episodes will appear as they air or are indexed.',
                        style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                      ),
                    ),
                  ],
                ),
              )
            else
              Wrap(
                spacing: 10,
                runSpacing: 10,
                children: List.generate(
                  show.episodes.isNotEmpty ? show.episodes.length : show.totalEpisodes,
                  (index) {
                    final epNumber = index + 1;
                    final isDone = show.episodes.isNotEmpty
                        ? show.episodes[index].isDownloaded
                        : epNumber <= show.downloadedEpisodes;

                    return Tooltip(
                      message: isDone
                          ? 'EP $epNumber is already organized. Click to re-download.'
                          : 'Click to download EP $epNumber',
                      child: InkWell(
                        onTap: () => state.downloadEpisode(show.malId, epNumber),
                        borderRadius: BorderRadius.circular(8),
                        child: Container(
                          width: 90,
                          padding: const EdgeInsets.symmetric(vertical: 10),
                          decoration: BoxDecoration(
                            color: isDone
                                ? ObsidianColors.surfaceContainerHigh
                                : ObsidianColors.surfaceContainerLowest,
                            borderRadius: BorderRadius.circular(8),
                            border: Border.all(
                              color: isDone
                                  ? ObsidianColors.primary.withOpacity(0.3)
                                  : ObsidianColors.border,
                            ),
                          ),
                          child: Column(
                            children: [
                              Text(
                                'EP $epNumber',
                                style: ObsidianTypography.monoMetric(
                                  color: isDone
                                      ? ObsidianColors.primary
                                      : ObsidianColors.outline,
                                ),
                              ),
                              const SizedBox(height: 4),
                              Icon(
                                isDone
                                    ? Icons.check_circle_rounded
                                    : Icons.arrow_downward_rounded,
                                size: 14,
                                color: isDone
                                    ? ObsidianColors.success
                                    : ObsidianColors.outline,
                              ),
                            ],
                          ),
                        ),
                      ),
                    );
                  },
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _MetaBadge extends StatelessWidget {
  final String label;
  final Color color;

  const _MetaBadge({required this.label, required this.color});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color.withOpacity(0.3)),
      ),
      child: Text(
        label,
        style: ObsidianTypography.labelBadge(color: color),
      ),
    );
  }
}
