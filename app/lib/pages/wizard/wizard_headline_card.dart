import 'package:flutter/material.dart';

import '../../theme/colors.dart';
import '../../theme/typography.dart';

class WizardHeadlineCard extends StatelessWidget {
  const WizardHeadlineCard({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    const Icon(
                      Icons.sync_alt_rounded,
                      size: 16,
                      color: ObsidianColors.primary,
                    ),
                    const SizedBox(width: 6),
                    Text(
                      'QUEUE PIPELINE ANALYSIS',
                      style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.primary,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 6),
                RichText(
                  text: TextSpan(
                    style: ObsidianTypography.headlineLg(),
                    children: const [
                      TextSpan(text: 'This will queue '),
                      TextSpan(
                        text: '214 episodes',
                        style: TextStyle(
                          color: ObsidianColors.primary,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      TextSpan(text: ' across '),
                      TextSpan(
                        text: '31 shows',
                        style: TextStyle(
                          color: ObsidianColors.tertiary,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 10),
                Wrap(
                  spacing: 6,
                  runSpacing: 4,
                  children: [
                    _buildPill(
                      '31 Tracked Series',
                      ObsidianColors.surfaceContainerHigh,
                      ObsidianColors.onSurface,
                    ),
                    _buildPill(
                      '214 Missing Backlog',
                      ObsidianColors.secondary.withOpacity(0.2),
                      ObsidianColors.secondary,
                    ),
                    _buildPill(
                      'Est. Storage ~248 GB',
                      ObsidianColors.tertiary.withOpacity(0.2),
                      ObsidianColors.tertiary,
                    ),
                    _buildPill(
                      'Nyaa.si (Gate Limiter)',
                      ObsidianColors.surfaceContainerHighest,
                      ObsidianColors.onSurfaceVariant,
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(width: 16),
          // Micro Telemetry Sparkline
          Container(
            width: 180,
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Flexible(
                      child: Text(
                        'Index Snatches',
                        style: ObsidianTypography.labelBadge(
                          color: ObsidianColors.outline,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    const SizedBox(width: 4),
                    Text(
                      '100%',
                      style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.tertiary,
                      ).copyWith(fontWeight: FontWeight.bold),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  crossAxisAlignment: CrossAxisAlignment.end,
                  children: [
                    _buildBar(10, ObsidianColors.surfaceContainerHigh),
                    _buildBar(18, ObsidianColors.primaryContainer),
                    _buildBar(26, ObsidianColors.primary),
                    _buildBar(22, ObsidianColors.primary),
                    _buildBar(30, ObsidianColors.tertiary),
                    _buildBar(14, ObsidianColors.tertiary),
                    _buildBar(25, ObsidianColors.secondary),
                    _buildBar(30, ObsidianColors.primaryContainer),
                  ],
                ),
                const SizedBox(height: 6),
                Align(
                  alignment: Alignment.centerRight,
                  child: Text(
                    '31/31 metadata resolved',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildBar(double height, Color color) {
    return Container(
      width: 10,
      height: height,
      decoration: BoxDecoration(
        color: color,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(2)),
      ),
    );
  }

  Widget _buildPill(String text, Color bg, Color textCol) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(text, style: ObsidianTypography.labelBadge(color: textCol)),
    );
  }
}
