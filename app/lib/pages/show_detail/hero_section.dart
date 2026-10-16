import 'package:flutter/material.dart';

import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'synopsis_section.dart';

class ShowDetailHero extends StatelessWidget {
  final StateProvider state;
  final LibraryShow show;
  final bool isMovie;
  final List<String> genres;

  const ShowDetailHero({
    super.key,
    required this.state,
    required this.show,
    required this.isMovie,
    required this.genres,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(24),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(16),
        boxShadow: const [
          BoxShadow(color: Colors.black26, blurRadius: 8, offset: Offset(0, 2)),
        ],
      ),
      child: Stack(
        children: [
          // Ambient decorative glow in top-right
          Positioned(
            right: -60,
            top: -60,
            child: Container(
              width: 200,
              height: 200,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: ObsidianColors.primary.withOpacity(0.04),
              ),
            ),
          ),

          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Title & Japanese Title
              Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 12,
                runSpacing: 6,
                children: [
                  Text(
                    show.englishTitle ?? show.title,
                    style: ObsidianTypography.headlineXl(),
                  ),
                  if (show.japaneseTitle != null &&
                      show.japaneseTitle!.isNotEmpty)
                    Text(
                      show.japaneseTitle!,
                      style: ObsidianTypography.headlineMd(
                        color: ObsidianColors.outline,
                      ),
                    ),
                ],
              ),
              const SizedBox(height: 6),

              // Subtitle metadata line
              Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 8,
                children: [
                  Text(
                    show.title,
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.primary,
                    ).copyWith(fontWeight: FontWeight.w500),
                  ),
                  Text(
                    '•',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.outline,
                    ),
                  ),
                  Text.rich(
                    TextSpan(
                      text: 'Studio: ',
                      style: ObsidianTypography.bodySm(
                        color: ObsidianColors.onSurfaceVariant,
                      ),
                      children: [
                        TextSpan(
                          text: show.studio ?? 'Unknown Studio',
                          style: ObsidianTypography.bodySm(
                            color: ObsidianColors.onSurface,
                          ).copyWith(fontWeight: FontWeight.bold),
                        ),
                      ],
                    ),
                  ),
                  Text(
                    '•',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.outline,
                    ),
                  ),
                  Text(
                    isMovie ? 'Original Story / Film' : 'TV Broadcast Series',
                    style: ObsidianTypography.bodySm(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 14),

              // Synopsis Section
              ShowDetailSynopsis(state: state, show: show),
              const SizedBox(height: 16),

              // Genre Chips
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: genres.map((g) {
                  return Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 5,
                    ),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerHigh,
                      borderRadius: BorderRadius.circular(20),
                    ),
                    child: Text(
                      g,
                      style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.onSurface,
                      ),
                    ),
                  );
                }).toList(),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
