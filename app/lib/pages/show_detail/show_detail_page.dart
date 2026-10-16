import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import 'aliases_tab.dart';
import 'episodes_tab.dart';
import 'hero_section.dart';
import 'left_rail.dart';
import 'related_works_section.dart';
import 'rules_tab.dart';
import 'show_detail_utils.dart';
import 'sub_header.dart';
import 'tab_bar.dart';

export 'tab_bar.dart' show ShowDetailTab;

class ShowDetailPage extends StatefulWidget {
  final LibraryShow show;

  const ShowDetailPage({super.key, required this.show});

  @override
  State<ShowDetailPage> createState() => _ShowDetailPageState();
}

class _ShowDetailPageState extends State<ShowDetailPage> {
  ShowDetailTab _activeTab = ShowDetailTab.episodes;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<StateProvider>().loadShowDetail(widget.show.malId);
    });
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();
    final show = state.selectedShow ?? widget.show;

    final onDiskString = show.onDiskBytes > 0
        ? formatBytes(show.onDiskBytes)
        : (show.downloadedEpisodes > 0
              ? '${(show.downloadedEpisodes * 1.4).toStringAsFixed(2)} GiB'
              : '0 B');

    final isMovie = show.mediaType.toLowerCase() == 'movie';
    final formatLabel = isMovie
        ? 'Movie (1 Part)'
        : 'TV Series (${show.totalEpisodes} Parts)';
    final durationLabel = isMovie
        ? 'Feature Film'
        : '${show.totalEpisodes} eps';
    final premieredLabel = show.startYear > 0 ? '${show.startYear}' : '-';
    final isFinished = show.airingStatus.toLowerCase().contains('finish');

    final effectiveGenres = show.genres;

    final effectiveAliases = show.aliases.isNotEmpty
        ? show.aliases
        : [
            show.title,
            if (show.englishTitle != null && show.englishTitle != show.title)
              show.englishTitle!,
            if (show.japaneseTitle != null) show.japaneseTitle!,
            if (show.startYear > 0) '${show.title} (${show.startYear})',
          ];

    return Material(
      color: ObsidianColors.surface,
      child: Column(
        children: [
          // Sub-Header & Breadcrumb Bar
          ShowDetailSubHeader(show: show, isMovie: isMovie),

          // Main Scrollable Area
          Expanded(
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(24),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // Main Content Layout Split: 320px Left Rail / Fluid Right Column
                  LayoutBuilder(
                    builder: (context, constraints) {
                      final isWide = constraints.maxWidth >= 980;
                      if (isWide) {
                        return Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            // Left Rail (320px)
                            SizedBox(
                              width: 320,
                              child: ShowDetailLeftRail(
                                state: state,
                                show: show,
                                isMovie: isMovie,
                                formatLabel: formatLabel,
                                durationLabel: durationLabel,
                                premieredLabel: premieredLabel,
                                isFinished: isFinished,
                                onDiskString: onDiskString,
                              ),
                            ),
                            const SizedBox(width: 24),

                            // Right Main Workspace
                            Expanded(
                              child: _buildRightWorkspace(
                                context,
                                state,
                                show,
                                isMovie,
                                effectiveGenres,
                                effectiveAliases,
                              ),
                            ),
                          ],
                        );
                      } else {
                        // Stacked layout on narrower viewports
                        return Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            ShowDetailLeftRail(
                              state: state,
                              show: show,
                              isMovie: isMovie,
                              formatLabel: formatLabel,
                              durationLabel: durationLabel,
                              premieredLabel: premieredLabel,
                              isFinished: isFinished,
                              onDiskString: onDiskString,
                            ),
                            const SizedBox(height: 24),
                            _buildRightWorkspace(
                              context,
                              state,
                              show,
                              isMovie,
                              effectiveGenres,
                              effectiveAliases,
                            ),
                          ],
                        );
                      }
                    },
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildRightWorkspace(
    BuildContext context,
    StateProvider state,
    LibraryShow show,
    bool isMovie,
    List<String> effectiveGenres,
    List<String> effectiveAliases,
  ) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Hero Information Card
        ShowDetailHero(
          state: state,
          show: show,
          isMovie: isMovie,
          genres: effectiveGenres,
        ),
        const SizedBox(height: 20),

        // 3-Tab Segmented Navigation Bar
        ShowDetailTabBar(
          show: show,
          activeTab: _activeTab,
          onTabChanged: (tab) => setState(() => _activeTab = tab),
        ),
        const SizedBox(height: 16),

        // Active Tab View
        if (_activeTab == ShowDetailTab.episodes)
          ShowDetailEpisodesTab(show: show, isMovie: isMovie)
        else if (_activeTab == ShowDetailTab.aliases)
          ShowDetailAliasesTab(show: show, aliases: effectiveAliases)
        else
          ShowDetailRulesTab(show: show),

        const SizedBox(height: 28),

        // Associated Related Works & Recommendations Bento Row
        ShowDetailRelatedWorks(show: show),
      ],
    );
  }
}
