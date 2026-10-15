import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../widgets/anime_poster.dart';

class ShowDetailRelatedWorks extends StatelessWidget {
  final LibraryShow show;

  const ShowDetailRelatedWorks({
    super.key,
    required this.show,
  });

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();
    final related = state.shows.where((s) => s.malId != show.malId).take(4).toList();
    if (related.isEmpty) return const SizedBox.shrink();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Wrap(
          alignment: WrapAlignment.spaceBetween,
          crossAxisAlignment: WrapCrossAlignment.center,
          spacing: 12,
          runSpacing: 6,
          children: [
            Wrap(
              crossAxisAlignment: WrapCrossAlignment.center,
              spacing: 10,
              children: [
                Text('Other Shows in Library', style: ObsidianTypography.titleSm()),
                Text(
                  '${related.length} available',
                  style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 11),
                ),
              ],
            ),
            InkWell(
              onTap: () {
                state.selectPage(3); // Library page
              },
              child: Text(
                'View All Shows →',
                style: ObsidianTypography.monoCode(color: ObsidianColors.primary),
              ),
            ),
          ],
        ),
        const SizedBox(height: 12),

        SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: Row(
            children: related.map((s) {
              return Padding(
                padding: const EdgeInsets.only(right: 12.0),
                child: SizedBox(
                  width: 220,
                  child: InkWell(
                    onTap: () => state.openShowDetail(s),
                    borderRadius: BorderRadius.circular(8),
                    child: _buildBentoCard(
                      title: s.title,
                      year: '${s.startYear > 0 ? s.startYear : '-'} • In Library',
                      badge: s.mediaType.toUpperCase(),
                      posterUrl: s.posterUrl ?? '',
                    ),
                  ),
                ),
              );
            }).toList(),
          ),
        ),
      ],
    );
  }

  Widget _buildBentoCard({
    required String title,
    required String year,
    required String badge,
    required String posterUrl,
  }) {
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        children: [
          ClipRRect(
            borderRadius: BorderRadius.circular(6),
            child: SizedBox(
              width: 44,
              height: 60,
              child: AnimePoster(
                urlOrPath: posterUrl,
                aspectRatio: 2 / 3,
                borderRadius: BorderRadius.circular(6),
              ),
            ),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  title,
                  style: ObsidianTypography.titleSm().copyWith(fontSize: 12, fontWeight: FontWeight.bold),
                  overflow: TextOverflow.ellipsis,
                ),
                Text(year, style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 10)),
                const SizedBox(height: 2),
                Text(badge, style: ObsidianTypography.labelBadge(color: ObsidianColors.success).copyWith(fontSize: 9)),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
