import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

/// Pattern Card B: Skeleton & Shimmer Card
class PatternCardB extends StatelessWidget {
  const PatternCardB({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border, width: 0.5),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text('Skeleton & Shimmer Card',
                    style: ObsidianTypography.titleSm(),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('POSTER GRID',
                    style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.outline)),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            'Used when fetching remote posters from AniList and parsing MAL seasonal synopses.',
            style: ObsidianTypography.bodySm(
                color: ObsidianColors.onSurfaceVariant),
          ),
          const Spacer(),

          // Showcase Canvas (2:3 preview dummy)
          Container(
            padding: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            alignment: Alignment.center,
            child: Container(
              width: 72,
              height: 100,
              padding: const EdgeInsets.all(6),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerHigh,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.end,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Container(
                    width: 36,
                    height: 6,
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceBright,
                      borderRadius: BorderRadius.circular(2),
                    ),
                  ),
                  const SizedBox(height: 4),
                  Container(
                    width: 50,
                    height: 8,
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceBright,
                      borderRadius: BorderRadius.circular(2),
                    ),
                  ),
                  const SizedBox(height: 4),
                  Container(
                    width: 24,
                    height: 6,
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceBright,
                      borderRadius: BorderRadius.circular(2),
                    ),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 10),

          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Flexible(
                child: Text('Ratio: 2:3 Portrait',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 4),
              Flexible(
                child: Text('Shimmer Pass',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.secondary, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
