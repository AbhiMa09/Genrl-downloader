import 'dart:io';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/library_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/state_views.dart';

enum PosterDensity { compact, normal, large }

class LibraryPage extends StatefulWidget {
  const LibraryPage({super.key});

  @override
  State<LibraryPage> createState() => _LibraryPageState();
}

class _LibraryPageState extends State<LibraryPage> {
  String _selectedStatus = 'All';
  String _selectedSort = 'updated';
  PosterDensity _density = PosterDensity.normal;
  final _searchController = TextEditingController();

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  void _openFileExplorer(String path) {
    if (!Platform.environment.containsKey('FLUTTER_TEST')) {
      Process.run('xdg-open', [path]);
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    if (state.coreError != null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24.0),
          child: ErrorStateView(
            title: 'Library Sync Error',
            message: state.coreError!,
            onRetry: () => state.refreshAll(),
            retryLabel: 'Retry Library Query',
          ),
        ),
      );
    }

    if (state.shows.isEmpty && !state.isMalAuthenticated) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24.0),
          child: EmptyStateView(
            title: 'MyAnimeList Not Connected',
            description: 'Connect your MyAnimeList account to sync your plan to watch and watching lists automatically.',
            action: ElevatedButton(
              onPressed: () => state.selectPage(5),
              child: const Text('Connect MAL in Settings'),
            ),
          ),
        ),
      );
    }

    final allShows = state.shows;

    // Compute status counts
    final watchingCount = allShows.where((s) => s.status.toLowerCase().contains('watch') && !s.status.toLowerCase().contains('plan')).length;
    final planCount = allShows.where((s) => s.status.toLowerCase().contains('plan')).length;
    final completedCount = allShows.where((s) => s.status.toLowerCase().contains('complet')).length;
    final incompleteCount = allShows.where((s) => s.episodesDownloaded < s.episodesTotal).length;
    final ongoingCount = allShows.where((s) => s.airingStatus.toLowerCase().contains('air') || s.status.toLowerCase().contains('watch')).length;

    // Filter shows
    var filteredShows = allShows.where((show) {
      if (_selectedStatus != 'All') {
        final norm = _selectedStatus.toLowerCase();
        final sNorm = show.status.toLowerCase();
        if (norm == 'watching' && (!sNorm.contains('watch') || sNorm.contains('plan'))) return false;
        if (norm == 'plan to watch' && !sNorm.contains('plan')) return false;
        if (norm == 'ongoing' && (!show.airingStatus.toLowerCase().contains('air') && !sNorm.contains('watch'))) return false;
        if (norm == 'completed' && !sNorm.contains('complet')) return false;
        if (norm == 'incomplete' && show.episodesDownloaded >= show.episodesTotal) return false;
      }

      final query = _searchController.text.trim().toLowerCase();
      if (query.isNotEmpty) {
        final matches = show.title.toLowerCase().contains(query) ||
            (show.englishTitle?.toLowerCase().contains(query) ?? false) ||
            (show.japaneseTitle?.toLowerCase().contains(query) ?? false);
        if (!matches) return false;
      }
      return true;
    }).toList();

    // Sort shows
    filteredShows.sort((a, b) {
      switch (_selectedSort) {
        case 'title':
          return a.title.compareTo(b.title);
        case 'score':
          return b.score.compareTo(a.score);
        case 'progress':
          final aProg = a.totalEpisodes > 0 ? a.downloadedEpisodes / a.totalEpisodes : 0.0;
          final bProg = b.totalEpisodes > 0 ? b.downloadedEpisodes / b.totalEpisodes : 0.0;
          return bProg.compareTo(aProg);
        case 'updated':
        default:
          return (b.malId).compareTo(a.malId);
      }
    });

    final totalTb = (allShows.fold<num>(0, (sum, s) => sum + s.onDiskBytes) / (1024 * 1024 * 1024 * 1024)).toStringAsFixed(1);

    return SingleChildScrollView(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // 1. Sub-header Telemetry Banner
          _buildTelemetryBanner(allShows.length, totalTb),

          // 2. Primary Workspace Control Bar
          _buildControlBar(
            totalCount: allShows.length,
            watchingCount: watchingCount,
            planCount: planCount,
            ongoingCount: ongoingCount,
            completedCount: completedCount,
            incompleteCount: incompleteCount,
            onRescan: () => state.manualDiskScan(),
            onAddSeries: () => state.selectPage(4),
          ),

          // 3. Main Grid Area
          Padding(
            padding: const EdgeInsets.all(24),
            child: filteredShows.isEmpty
                ? const EmptyStateView(
                    icon: Icons.video_library_outlined,
                    title: 'No Anime Matches Filter',
                    description: 'Try adjusting your search query or status filter chips.',
                  )
                : LayoutBuilder(
                    builder: (context, constraints) {
                      int columns;
                      switch (_density) {
                        case PosterDensity.compact:
                          columns = constraints.maxWidth > 1300
                              ? 8
                              : constraints.maxWidth > 900
                                  ? 6
                                  : 4;
                          break;
                        case PosterDensity.large:
                          columns = constraints.maxWidth > 1300
                              ? 4
                              : constraints.maxWidth > 900
                                  ? 3
                                  : 2;
                          break;
                        case PosterDensity.normal:
                          columns = constraints.maxWidth > 1300
                              ? 6
                              : constraints.maxWidth > 900
                                  ? 4
                                  : constraints.maxWidth > 600
                                      ? 3
                                      : 2;
                          break;
                      }

                      final itemWidth = (constraints.maxWidth - (columns - 1) * 16) / columns;

                      return Wrap(
                        spacing: 16,
                        runSpacing: 16,
                        children: filteredShows.map((show) {
                          return SizedBox(
                            width: itemWidth,
                            child: _LibraryPosterCard(
                              show: show,
                              density: _density,
                              onTap: () => state.openShowDetail(show),
                              onOpenFolder: () => _openFileExplorer(show.folderPath ?? '/mnt/anime/TV/${show.title}'),
                              onToggleAuto: () {},
                            ),
                          );
                        }).toList(),
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }

  Widget _buildTelemetryBanner(int seriesCount, String totalTb) {
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
                  Text('Anime Library', style: ObsidianTypography.headlineMd(fontSize: 16)),
                  const SizedBox(width: 12),
                  Container(width: 1, height: 14, color: ObsidianColors.border),
                  const SizedBox(width: 12),
                  const Icon(Icons.folder_copy_rounded, size: 16, color: ObsidianColors.primary),
                  const SizedBox(width: 6),
                  Text(
                    '$seriesCount Series',
                    style: ObsidianTypography.monoMetric(color: ObsidianColors.onSurface, fontWeight: FontWeight.bold, fontSize: 12),
                  ),
                  Text(' / ', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                  Text(
                    '$totalTb TB on /mnt/anime/TV',
                    style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 11),
                  ),
                ],
              ),
              Container(width: 1, height: 12, color: ObsidianColors.border),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Container(width: 6, height: 6, decoration: const BoxDecoration(color: ObsidianColors.tertiary, shape: BoxShape.circle)),
                  const SizedBox(width: 6),
                  Text('All Folders Monitored', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 11)),
                ],
              ),
              Container(width: 1, height: 12, color: ObsidianColors.border),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(Icons.sync_rounded, size: 14, color: ObsidianColors.tertiary),
                  const SizedBox(width: 6),
                  Text('Inotify Kernel Watcher Active', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 11)),
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
                child: Text('FS: Btrfs (compress=zstd:3)', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10)),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('qB: v4.6.3 headless', style: ObsidianTypography.monoCode(color: ObsidianColors.tertiary, fontSize: 10)),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildControlBar({
    required int totalCount,
    required int watchingCount,
    required int planCount,
    required int ongoingCount,
    required int completedCount,
    required int incompleteCount,
    required VoidCallback onRescan,
    required VoidCallback onAddSeries,
  }) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
      decoration: const BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        border: Border(bottom: BorderSide(color: ObsidianColors.border)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Wrap(
            alignment: WrapAlignment.spaceBetween,
            crossAxisAlignment: WrapCrossAlignment.center,
            spacing: 16,
            runSpacing: 10,
            children: [
              // Search Input & Filter Chips
              Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 12,
                runSpacing: 8,
                children: [
                  // Filter input box
                  Container(
                    width: 260,
                    height: 36,
                    padding: const EdgeInsets.symmetric(horizontal: 10),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerLowest,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: ObsidianColors.border),
                    ),
                    child: Row(
                      children: [
                        const Icon(Icons.filter_list_rounded, size: 16, color: ObsidianColors.outline),
                        const SizedBox(width: 8),
                        Expanded(
                          child: TextField(
                            controller: _searchController,
                            style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
                            decoration: InputDecoration(
                              hintText: 'Filter library... (Ctrl+F)',
                              hintStyle: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                              border: InputBorder.none,
                              isDense: true,
                            ),
                            onChanged: (_) => setState(() {}),
                          ),
                        ),
                        Container(
                          padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1.5),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainerHighest,
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Text('Ctrl+F', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 9)),
                        ),
                      ],
                    ),
                  ),

                  // Filter Chips
                  _buildChip('All', '$totalCount'),
                  _buildChip('Watching', '$watchingCount', activeColor: ObsidianColors.tertiary),
                  _buildChip('Plan to Watch', '$planCount'),
                  _buildChip('Ongoing', '$ongoingCount', activeColor: ObsidianColors.tertiary),
                  _buildChip('Completed', '$completedCount'),
                  _buildChip('Incomplete', '$incompleteCount', activeColor: ObsidianColors.error),
                ],
              ),

              // Sort, Density & Actions Section
              Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 12,
                runSpacing: 8,
                children: [
                  // Sort Dropdown
                  Container(
                    height: 34,
                    padding: const EdgeInsets.symmetric(horizontal: 8),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainer,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: ObsidianColors.border),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text('SORT: ', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline, fontSize: 9)),
                        DropdownButtonHideUnderline(
                          child: DropdownButton<String>(
                            value: _selectedSort,
                            dropdownColor: ObsidianColors.surfaceContainerHigh,
                            style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface, fontSize: 11),
                            items: const [
                              DropdownMenuItem(value: 'updated', child: Text('Last Updated')),
                              DropdownMenuItem(value: 'title', child: Text('Title (A-Z)')),
                              DropdownMenuItem(value: 'score', child: Text('Community Score')),
                              DropdownMenuItem(value: 'progress', child: Text('Completion Progress')),
                            ],
                            onChanged: (val) {
                              if (val != null) setState(() => _selectedSort = val);
                            },
                          ),
                        ),
                      ],
                    ),
                  ),

                  // Poster Size Toggle
                  Container(
                    height: 34,
                    padding: const EdgeInsets.all(2),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerLowest,
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: ObsidianColors.border),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        _buildDensityButton('Compact', PosterDensity.compact),
                        _buildDensityButton('Normal', PosterDensity.normal),
                        _buildDensityButton('Large', PosterDensity.large),
                      ],
                    ),
                  ),

                  // Rescan & Add Series Buttons
                  OutlinedButton.icon(
                    onPressed: onRescan,
                    icon: const Icon(Icons.autorenew_rounded, size: 16, color: ObsidianColors.tertiary),
                    label: Text('Rescan folders', style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface, fontSize: 11)),
                    style: OutlinedButton.styleFrom(
                      backgroundColor: ObsidianColors.surfaceContainer,
                      side: const BorderSide(color: ObsidianColors.border),
                      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                    ),
                  ),

                  ElevatedButton.icon(
                    onPressed: onAddSeries,
                    icon: const Icon(Icons.add_rounded, size: 16),
                    label: Text('Add Series', style: ObsidianTypography.titleSm(color: ObsidianColors.onPrimary, fontSize: 12)),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: ObsidianColors.primary,
                      foregroundColor: ObsidianColors.onPrimary,
                      elevation: 0,
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildChip(String label, String count, {Color? activeColor}) {
    final isSelected = _selectedStatus.toLowerCase() == label.toLowerCase();
    return InkWell(
      onTap: () => setState(() => _selectedStatus = label),
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: isSelected ? ObsidianColors.primaryContainer : ObsidianColors.surfaceContainerHigh,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isSelected ? ObsidianColors.primary : ObsidianColors.border,
            width: isSelected ? 1 : 0.5,
          ),
        ),
        child: Text(
          '$label ($count)',
          style: ObsidianTypography.monoCode(
            color: isSelected
                ? ObsidianColors.onPrimaryContainer
                : (activeColor ?? ObsidianColors.onSurfaceVariant),
            fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
            fontSize: 11,
          ),
        ),
      ),
    );
  }

  Widget _buildDensityButton(String label, PosterDensity density) {
    final isSelected = _density == density;
    return InkWell(
      onTap: () => setState(() => _density = density),
      borderRadius: BorderRadius.circular(6),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          color: isSelected ? ObsidianColors.surfaceContainerHighest : Colors.transparent,
          borderRadius: BorderRadius.circular(6),
        ),
        child: Text(
          label,
          style: ObsidianTypography.monoCode(
            color: isSelected ? ObsidianColors.primary : ObsidianColors.outline,
            fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
            fontSize: 10,
          ),
        ),
      ),
    );
  }
}

class _LibraryPosterCard extends StatefulWidget {
  final LibraryShow show;
  final PosterDensity density;
  final VoidCallback onTap;
  final VoidCallback onOpenFolder;
  final VoidCallback onToggleAuto;

  const _LibraryPosterCard({
    required this.show,
    required this.density,
    required this.onTap,
    required this.onOpenFolder,
    required this.onToggleAuto,
  });

  @override
  State<_LibraryPosterCard> createState() => _LibraryPosterCardState();
}

class _LibraryPosterCardState extends State<_LibraryPosterCard> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final show = widget.show;
    final downloaded = show.episodesDownloaded;
    final total = show.episodesTotal;
    final percent = (downloaded / (total > 0 ? total : 1)).clamp(0.0, 1.0);
    final percentInt = (percent * 100).toInt();

    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: InkWell(
        onTap: widget.onTap,
        borderRadius: BorderRadius.circular(12),
        child: Container(
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainerLow,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(
              color: _isHovered ? ObsidianColors.primary.withOpacity(0.6) : ObsidianColors.border,
            ),
            boxShadow: _isHovered
                ? [
                    BoxShadow(
                      color: ObsidianColors.primary.withOpacity(0.2),
                      blurRadius: 16,
                      offset: const Offset(0, 4),
                    ),
                  ]
                : null,
          ),
          clipBehavior: Clip.antiAlias,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // 2:3 Poster
              AspectRatio(
                aspectRatio: 2 / 3,
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    if (show.posterUrl != null && show.posterUrl!.isNotEmpty)
                      Image.network(
                        show.posterUrl!,
                        fit: BoxFit.cover,
                        errorBuilder: (_, __, ___) => _buildPlaceholder(show),
                      )
                    else
                      _buildPlaceholder(show),

                    // Scrim
                    Container(
                      decoration: const BoxDecoration(
                        gradient: LinearGradient(
                          begin: Alignment.topCenter,
                          end: Alignment.bottomCenter,
                          stops: [0.0, 0.4, 1.0],
                          colors: [
                            Colors.black38,
                            Colors.transparent,
                            Colors.black87,
                          ],
                        ),
                      ),
                    ),

                    // Top Left Badges
                    Positioned(
                      top: 6,
                      left: 6,
                      child: Row(
                        children: [
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 2),
                            decoration: BoxDecoration(
                              color: ObsidianColors.secondaryContainer.withOpacity(0.9),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text('AUTO', style: ObsidianTypography.labelBadge(color: ObsidianColors.onSecondaryContainer, fontSize: 8)),
                          ),
                          const SizedBox(width: 4),
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 2),
                            decoration: BoxDecoration(
                              color: show.status == 'Completed'
                                  ? ObsidianColors.surfaceContainerHighest.withOpacity(0.85)
                                  : ObsidianColors.tertiaryContainer.withOpacity(0.85),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text(
                              show.status == 'Completed' ? 'COMPLETED' : 'AIRING',
                              style: ObsidianTypography.labelBadge(
                                color: show.status == 'Completed' ? ObsidianColors.onSurfaceVariant : ObsidianColors.onTertiary,
                                fontSize: 8,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),

                    // Top Right Quick Hover Action
                    if (_isHovered)
                      Positioned(
                        top: 6,
                        right: 6,
                        child: Container(
                          padding: const EdgeInsets.all(2),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainerHighest.withOpacity(0.9),
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Row(
                            children: [
                              InkWell(
                                onTap: widget.onOpenFolder,
                                child: const Padding(
                                  padding: EdgeInsets.all(3.0),
                                  child: Icon(Icons.folder_open_rounded, size: 14, color: ObsidianColors.onSurfaceVariant),
                                ),
                              ),
                              InkWell(
                                onTap: widget.onToggleAuto,
                                child: const Padding(
                                  padding: EdgeInsets.all(3.0),
                                  child: Icon(Icons.bolt_rounded, size: 14, color: ObsidianColors.onSurfaceVariant),
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),

                    // Bottom Quality Badge
                    Positioned(
                      bottom: 6,
                      left: 6,
                      right: 6,
                      child: Text(
                        '1080p HEVC · Erai',
                        style: ObsidianTypography.monoCode(color: ObsidianColors.tertiary, fontSize: 9, fontWeight: FontWeight.bold),
                      ),
                    ),
                  ],
                ),
              ),

              // Card Bottom Metadata
              Padding(
                padding: const EdgeInsets.all(10),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      show.title,
                      style: ObsidianTypography.titleSm(fontSize: widget.density == PosterDensity.compact ? 11 : 13),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 2),
                    Text(
                      'Season ${show.season} · ${show.studio ?? "Anime"}',
                      style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 8),

                    // Progress Bar & Counter
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Text('$downloaded / $total eps', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 10)),
                        Text('$percentInt%', style: ObsidianTypography.monoMetric(color: ObsidianColors.tertiary, fontWeight: FontWeight.bold, fontSize: 10)),
                      ],
                    ),
                    const SizedBox(height: 4),
                    ClipRRect(
                      borderRadius: BorderRadius.circular(2),
                      child: LinearProgressIndicator(
                        value: percent,
                        minHeight: 4,
                        backgroundColor: ObsidianColors.surfaceContainerHighest,
                        valueColor: const AlwaysStoppedAnimation(ObsidianColors.primary),
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildPlaceholder(LibraryShow show) {
    return Container(
      color: ObsidianColors.surfaceContainerHighest,
      child: Center(
        child: Text(
          show.title.isNotEmpty ? show.title[0] : '?',
          style: ObsidianTypography.headlineLg(color: ObsidianColors.outline),
        ),
      ),
    );
  }
}
