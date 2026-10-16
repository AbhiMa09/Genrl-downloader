import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class WizardSeriesManifest extends StatelessWidget {
  final StateProvider state;

  const WizardSeriesManifest({super.key, required this.state});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Row(
                children: [
                  Text(
                    'Detected Series Manifest',
                    style: ObsidianTypography.titleSm().copyWith(
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 6,
                      vertical: 2,
                    ),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerHigh,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      '${state.shows.length} Shows Found',
                      style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.onSurfaceVariant,
                      ),
                    ),
                  ),
                ],
              ),
              Row(
                children: [
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 4,
                    ),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerLowest,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Row(
                      children: [
                        const Icon(
                          Icons.filter_list_rounded,
                          size: 14,
                          color: ObsidianColors.outline,
                        ),
                        const SizedBox(width: 4),
                        Text(
                          'Missing backlog only',
                          style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 12),
          Builder(
            builder: (context) {
              final detected = state.shows
                  .where((s) => s.downloadedEpisodes < s.totalEpisodes)
                  .take(4)
                  .map((s) {
                    final missingCount = s.totalEpisodes - s.downloadedEpisodes;
                    return {
                      'title': s.title,
                      'tier': s.score >= 8.5 ? 'Tier 1' : 'Tier 2',
                      'missing': 'Missing: $missingCount eps',
                      'spec': '[Auto] 1080p HEVC',
                      'size': '${(missingCount * 1.2).toStringAsFixed(1)} GB',
                      'status': 'Ready to queue',
                      'poster': s.posterUrl ?? '',
                    };
                  })
                  .toList();

              if (detected.isEmpty) {
                return Padding(
                  padding: const EdgeInsets.symmetric(vertical: 16),
                  child: Center(
                    child: Text(
                      'No backlog episodes detected',
                      style: ObsidianTypography.bodySm(
                        color: ObsidianColors.outline,
                      ),
                    ),
                  ),
                );
              }

              return ListView.separated(
                shrinkWrap: true,
                physics: const NeverScrollableScrollPhysics(),
                itemCount: detected.length,
                separatorBuilder: (_, __) => const SizedBox(height: 8),
                itemBuilder: (context, index) {
                  final item = detected[index];
                  return Container(
                    padding: const EdgeInsets.all(10),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerLow,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Row(
                      children: [
                        // 2:3 Cover Poster
                        Container(
                          width: 36,
                          height: 54,
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainerHighest,
                            borderRadius: BorderRadius.circular(4),
                          ),
                          clipBehavior: Clip.antiAlias,
                          child: Image.network(
                            item['poster'] as String,
                            fit: BoxFit.cover,
                            errorBuilder: (_, __, ___) => const Icon(
                              Icons.movie_rounded,
                              size: 18,
                              color: ObsidianColors.outline,
                            ),
                          ),
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Row(
                                children: [
                                  Text(
                                    item['title'] as String,
                                    style: ObsidianTypography.titleSm()
                                        .copyWith(fontWeight: FontWeight.bold),
                                  ),
                                  const SizedBox(width: 6),
                                  Container(
                                    padding: const EdgeInsets.symmetric(
                                      horizontal: 5,
                                      vertical: 1,
                                    ),
                                    decoration: BoxDecoration(
                                      color: ObsidianColors.primaryContainer,
                                      borderRadius: BorderRadius.circular(4),
                                    ),
                                    child: Text(
                                      item['tier'] as String,
                                      style: ObsidianTypography.labelBadge(
                                        color:
                                            ObsidianColors.onPrimaryContainer,
                                      ),
                                    ),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 4),
                              Row(
                                children: [
                                  Text(
                                    item['missing'] as String,
                                    style: ObsidianTypography.labelBadge(
                                      color: ObsidianColors.tertiary,
                                    ),
                                  ),
                                  const SizedBox(width: 6),
                                  Text(
                                    '•',
                                    style: ObsidianTypography.labelBadge(
                                      color: ObsidianColors.outline,
                                    ),
                                  ),
                                  const SizedBox(width: 6),
                                  Text(
                                    item['spec'] as String,
                                    style: ObsidianTypography.labelBadge(
                                      color: ObsidianColors.outline,
                                    ),
                                  ),
                                  const SizedBox(width: 6),
                                  Text(
                                    '•',
                                    style: ObsidianTypography.labelBadge(
                                      color: ObsidianColors.outline,
                                    ),
                                  ),
                                  const SizedBox(width: 6),
                                  Text(
                                    item['size'] as String,
                                    style: ObsidianTypography.labelBadge(
                                      color: ObsidianColors.outline,
                                    ),
                                  ),
                                ],
                              ),
                            ],
                          ),
                        ),
                        Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 8,
                            vertical: 4,
                          ),
                          decoration: BoxDecoration(
                            color: ObsidianColors.secondary.withOpacity(0.2),
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Row(
                            children: [
                              Container(
                                width: 5,
                                height: 5,
                                decoration: const BoxDecoration(
                                  shape: BoxShape.circle,
                                  color: ObsidianColors.secondary,
                                ),
                              ),
                              const SizedBox(width: 6),
                              Text(
                                'Ready to queue',
                                style: ObsidianTypography.labelBadge(
                                  color: ObsidianColors.secondary,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  );
                },
              );
            },
          ),
        ],
      ),
    );
  }
}
