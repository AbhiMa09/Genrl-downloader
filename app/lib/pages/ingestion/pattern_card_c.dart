import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

/// Pattern Card C: Packet Buffer & CRC32 Hash
class PatternCardC extends StatelessWidget {
  const PatternCardC({super.key});

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
                child: Text('Packet Buffer Check',
                    style: ObsidianTypography.titleSm(),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.secondaryContainer.withOpacity(0.4),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('TELEMETRY',
                    style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.secondary)),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            'Radar telemetry loader for CRC32 / MD5 block integrity while writing to /mnt/anime.',
            style: ObsidianTypography.bodySm(
                color: ObsidianColors.onSurfaceVariant),
          ),
          const Spacer(),

          // Showcase Canvas
          Container(
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              children: [
                Container(
                  width: 44,
                  height: 44,
                  decoration: const BoxDecoration(
                    color: ObsidianColors.surfaceContainerHigh,
                    shape: BoxShape.circle,
                  ),
                  child: const Icon(
                    Icons.radar_rounded,
                    color: ObsidianColors.primary,
                    size: 24,
                  ),
                ),
                const SizedBox(height: 6),
                Text(
                  'CRC32: 0x9B4E1A • 85.4 MB/s',
                  style: ObsidianTypography.monoCode(
                    color: ObsidianColors.primary,
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
                child: Text('Buffer: 64MB In-RAM',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 4),
              Flexible(
                child: Text('Zero-Drop IO',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.primary, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
