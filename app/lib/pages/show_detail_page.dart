import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/library_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/anime_poster.dart';

class ShowDetailPage extends StatelessWidget {
  final LibraryShow show;

  const ShowDetailPage({
    super.key,
    required this.show,
  });

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    return Scaffold(
      backgroundColor: ObsidianColors.surface,
      appBar: AppBar(
        backgroundColor: ObsidianColors.surfaceContainerLow,
        elevation: 0,
        leading: IconButton(
          icon: const Icon(Icons.arrow_back_rounded,
              color: ObsidianColors.onSurface),
          onPressed: () => state.closeShowDetail(),
        ),
        title: Text(show.title, style: ObsidianTypography.titleSm()),
        actions: [
          IconButton(
            icon: const Icon(Icons.tune_rounded, color: ObsidianColors.primary),
            tooltip: 'Configure Per-Show Rules',
            onPressed: () => state.selectPage(6), // Go to Rules
          ),
          const SizedBox(width: 8),
        ],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header Hero Section (Poster + Metadata)
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // 2:3 Poster
                SizedBox(
                  width: 160,
                  height: 240,
                  child: AnimePoster(
                    urlOrPath: show.posterUrl,
                    aspectRatio: 2 / 3,
                    borderRadius: BorderRadius.circular(10),
                  ),
                ),
                const SizedBox(width: 20),

                // Metadata Details
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(show.title, style: ObsidianTypography.headlineLg()),
                      if (show.englishTitle != null) ...[
                        const SizedBox(height: 4),
                        Text(
                          show.englishTitle!,
                          style: ObsidianTypography.bodyMd(
                              color: ObsidianColors.onSurfaceVariant),
                        ),
                      ],
                      const SizedBox(height: 12),

                      // Metric badges
                      Row(
                        children: [
                          _MetaBadge(
                            label: '★ ${show.score}',
                            color: ObsidianColors.primary,
                          ),
                          const SizedBox(width: 8),
                          _MetaBadge(
                            label: '${show.startYear}',
                            color: ObsidianColors.outline,
                          ),
                          const SizedBox(width: 8),
                          _MetaBadge(
                            label: '${show.totalEpisodes} Episodes',
                            color: ObsidianColors.tertiary,
                          ),
                          const SizedBox(width: 8),
                          _MetaBadge(
                            label: show.airingStatus,
                            color: ObsidianColors.success,
                          ),
                        ],
                      ),
                      const SizedBox(height: 12),

                      // Genres
                      Wrap(
                        spacing: 6,
                        runSpacing: 6,
                        children: show.genres.map((g) {
                          return Container(
                            padding: const EdgeInsets.symmetric(
                                horizontal: 8, vertical: 3),
                            decoration: BoxDecoration(
                              color: ObsidianColors.surfaceContainerHigh,
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text(
                              g,
                              style: ObsidianTypography.labelBadge(
                                  color: ObsidianColors.onSurfaceVariant),
                            ),
                          );
                        }).toList(),
                      ),
                      const SizedBox(height: 14),

                      // Folder Path display
                      if (show.folderPath != null)
                        Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 10, vertical: 6),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainerLowest,
                            borderRadius: BorderRadius.circular(6),
                            border: Border.all(
                                color: ObsidianColors.border, width: 0.5),
                          ),
                          child: Row(
                            children: [
                              const Icon(Icons.folder_open_rounded,
                                  size: 14, color: ObsidianColors.outline),
                              const SizedBox(width: 6),
                              Expanded(
                                child: Text(
                                  show.folderPath!,
                                  style: ObsidianTypography.monoCode(),
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                            ],
                          ),
                        ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 24),

            // Synopsis narrative block
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLow,
                borderRadius: BorderRadius.circular(10),
                border: Border.all(color: ObsidianColors.border),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('SYNOPSIS', style: ObsidianTypography.titleSm()),
                  const SizedBox(height: 8),
                  Text(
                    show.synopsis,
                    style: ObsidianTypography.bodyMd(),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 24),

            // Episodes Grid Matrix
            Text('EPISODES MATRIX', style: ObsidianTypography.titleSm()),
            const SizedBox(height: 12),

            Wrap(
              spacing: 10,
              runSpacing: 10,
              children: List.generate(show.totalEpisodes, (index) {
                final epNumber = index + 1;
                final isDone = epNumber <= show.downloadedEpisodes;

                return Container(
                  width: 90,
                  padding: const EdgeInsets.symmetric(vertical: 10),
                  decoration: BoxDecoration(
                    color: isDone
                        ? ObsidianColors.surfaceContainerHigh
                        : ObsidianColors.surfaceContainerLowest,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(
                      color: isDone
                          ? ObsidianColors.primary.withOpacity(0.3)
                          : ObsidianColors.border,
                    ),
                  ),
                  child: Column(
                    children: [
                      Text(
                        'EP $epNumber',
                        style: ObsidianTypography.monoMetric(
                          color: isDone
                              ? ObsidianColors.primary
                              : ObsidianColors.outline,
                        ),
                      ),
                      const SizedBox(height: 4),
                      Icon(
                        isDone
                            ? Icons.check_circle_rounded
                            : Icons.arrow_downward_rounded,
                        size: 14,
                        color: isDone
                            ? ObsidianColors.success
                            : ObsidianColors.outline,
                      ),
                    ],
                  ),
                );
              }),
            ),
          ],
        ),
      ),
    );
  }
}

class _MetaBadge extends StatelessWidget {
  final String label;
  final Color color;

  const _MetaBadge({required this.label, required this.color});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color.withOpacity(0.3)),
      ),
      child: Text(
        label,
        style: ObsidianTypography.labelBadge(color: color),
      ),
    );
  }
}
