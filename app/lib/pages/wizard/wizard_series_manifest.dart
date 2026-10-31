import 'package:flutter/material.dart';

import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

/// Review each watching season, including seasons with no planned downloads.
/// MAL IDs keep distinct seasons separate even when their display titles match.
class WizardSeriesManifest extends StatelessWidget {
  final StateProvider state;
  const WizardSeriesManifest({super.key, required this.state});

  static int seasonCount(StateProvider state) => _seasons(state).length;

  static List<_SeasonPlan> _seasons(StateProvider state) {
    final shows = {for (final show in state.shows) show.malId: show};
    final seasons = <int, _SeasonPlan>{
      for (final show in state.shows)
        if (show.status.toLowerCase() == 'watching')
          show.malId: _SeasonPlan(show.malId, show.title, show),
    };
    for (final episode in state.plannedEpisodesList) {
      final id = episode['mal_id'];
      if (id is! int) continue;
      final show = shows[id];
      final season = seasons.putIfAbsent(
        id,
        () => _SeasonPlan(
          id,
          show?.title ?? episode['title'] as String? ?? 'Untitled season',
          show,
        ),
      );
      final number = episode['episode'];
      if (number is num) season.episodes.add(number);
    }
    return seasons.values.toList();
  }

  @override
  Widget build(BuildContext context) {
    final seasons = _seasons(state);
    return RepaintBoundary(
      key: const ValueKey('manifest_repaint_boundary'),
      child: Container(
        padding: const EdgeInsets.all(14),
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainer,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: ObsidianColors.border),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              'Season review • ${seasons.length} seasons',
              style: ObsidianTypography.titleSm(),
            ),
            const SizedBox(height: 4),
            Text(
              'All watching seasons. Expand a season to review its planned episodes.',
              style: ObsidianTypography.bodySm(
                color: ObsidianColors.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 12),
            if (seasons.isEmpty)
              const Padding(
                padding: EdgeInsets.all(16),
                child: Text('No watching seasons or planned downloads.'),
              ),
            for (final season in seasons) _seasonTile(season),
          ],
        ),
      ),
    );
  }

  Widget _seasonTile(_SeasonPlan season) {
    final show = season.show;
    final episodes = season.episodes.toList()..sort();
    final seasonLabel = show != null && show.season > 0
        ? 'Season ${show.season}'
        : 'Season number unavailable';
    final metadata = show == null
        ? 'Metadata unavailable'
        : !show.metadataResolved
        ? 'Metadata unresolved'
        : show.metadataStatus == 'partial'
        ? 'Partial metadata'
        : 'Metadata ready';
    final total = show != null && show.totalEpisodes > 0
        ? '${show.totalEpisodes} episodes total'
        : 'Episode total unknown';
    final summary = episodes.isEmpty
        ? 'No episodes planned'
        : '${episodes.length} episodes planned';
    return Card(
      key: ValueKey('plan_season_${season.id}'),
      color: ObsidianColors.surfaceContainerLow,
      child: ExpansionTile(
        title: Text(season.title, style: ObsidianTypography.titleSm()),
        subtitle: Text(
          '$seasonLabel • $total\n$summary • $metadata',
          style: ObsidianTypography.bodySm(
            color: ObsidianColors.onSurfaceVariant,
          ),
        ),
        childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
        expandedCrossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (episodes.isEmpty)
            const Text(
              'This plan will create no download jobs for this season.',
            )
          else
            Wrap(
              spacing: 8,
              runSpacing: 4,
              children: [
                for (final number in episodes)
                  Chip(
                    label: Text(
                      'Episode ${number == number.toInt() ? number.toInt() : number}',
                    ),
                  ),
              ],
            ),
        ],
      ),
    );
  }
}

class _SeasonPlan {
  final int id;
  final String title;
  final LibraryShow? show;
  final Set<num> episodes = {};
  _SeasonPlan(this.id, this.title, this.show);
}
