import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class PatternCardA extends StatelessWidget {
  const PatternCardA({super.key});

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
                child: Text('Mini Inline Spinner',
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
                child: Text('TABLE / ROW',
                    style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.outline)),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            'Compressed feedback when re-indexing season folders or triggering single item hash checks.',
            style: ObsidianTypography.bodySm(
                color: ObsidianColors.onSurfaceVariant),
          ),
          const Spacer(),

          // Showcase Canvas
          Container(
            padding: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              children: [
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHigh,
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: Row(
                    children: [
                      const SizedBox(
                        width: 14,
                        height: 14,
                        child: CircularProgressIndicator(
                          strokeWidth: 2,
                          valueColor: AlwaysStoppedAnimation<Color>(
                              ObsidianColors.tertiary),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          'Ingestion Job #101 [1080p HEVC]',
                          style: ObsidianTypography.monoCode(),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      Text('POLL',
                          style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.tertiary)),
                    ],
                  ),
                ),
                const SizedBox(height: 6),
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer.withOpacity(0.4),
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: Row(
                    children: [
                      const Icon(Icons.check,
                          size: 14, color: ObsidianColors.outline),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          'Ingestion Job #100 [BDRip FLAC]',
                          style: ObsidianTypography.monoCode(
                              color: ObsidianColors.outline),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      Text('IDLE',
                          style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.outline)),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 10),

          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Flexible(
                child: Text('Latency: <4ms',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 4),
              Flexible(
                child: Text('CSS Keyframe',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.tertiary, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
