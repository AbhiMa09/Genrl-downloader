import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class SearchResultsTable extends StatelessWidget {
  final List<Map<String, dynamic>> searchResults;
  final ValueChanged<String> onGrabRelease;

  const SearchResultsTable({
    super.key,
    required this.searchResults,
    required this.onGrabRelease,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      clipBehavior: Clip.antiAlias,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Table Heading
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
            color: ObsidianColors.surfaceContainerHigh,
            child: Row(
              children: [
                Expanded(
                    flex: 6,
                    child: Text('TORRENT / RELEASE SPECIFICATION',
                        style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline))),
                Expanded(
                    flex: 2,
                    child: Text('GROUP',
                        style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline))),
                Expanded(
                    flex: 2,
                    child: Text('FORMAT',
                        style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline))),
                Expanded(
                    flex: 2,
                    child: Text('SIZE',
                        textAlign: TextAlign.right,
                        style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline))),
                Expanded(
                    flex: 2,
                    child: Text('SWARM (S/L)',
                        textAlign: TextAlign.center,
                        style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline))),
                Expanded(
                    flex: 1,
                    child: Text('AGE',
                        style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline))),
                Expanded(
                    flex: 2,
                    child: Text('SCORE',
                        style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline))),
                const SizedBox(
                    width: 80,
                    child: Text('ACTION',
                        textAlign: TextAlign.right,
                        style:
                            TextStyle(fontSize: 10, color: ObsidianColors.outline))),
              ],
            ),
          ),

          // Table Rows
          ListView.separated(
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            itemCount: searchResults.length,
            separatorBuilder: (_, __) =>
                const Divider(height: 1, color: ObsidianColors.border),
            itemBuilder: (context, index) {
              final r = searchResults[index];
              final isBest = r['isBest'] as bool;

              return Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                decoration: BoxDecoration(
                  color: isBest
                      ? ObsidianColors.surfaceContainer.withOpacity(0.8)
                      : Colors.transparent,
                  border: isBest
                      ? Border.all(
                          color: ObsidianColors.primary.withOpacity(0.4))
                      : null,
                ),
                child: Row(
                  children: [
                    // 1. Torrent Spec & Bonuses
                    Expanded(
                      flex: 6,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              if (isBest) ...[
                                Container(
                                  padding: const EdgeInsets.symmetric(
                                      horizontal: 5, vertical: 1.5),
                                  decoration: BoxDecoration(
                                      color: ObsidianColors.primary,
                                      borderRadius: BorderRadius.circular(4)),
                                  child: Row(
                                    mainAxisSize: MainAxisSize.min,
                                    children: [
                                      const Icon(Icons.star_rounded,
                                          size: 10,
                                          color: ObsidianColors.onPrimary),
                                      const SizedBox(width: 3),
                                      Text('Best Rule Match',
                                          style: ObsidianTypography.labelBadge(
                                              color: ObsidianColors.onPrimary,
                                              fontSize: 8)),
                                    ],
                                  ),
                                ),
                                const SizedBox(width: 6),
                              ],
                              Expanded(
                                child: Text(
                                  r['title'] as String,
                                  style: ObsidianTypography.monoCode(
                                    color: ObsidianColors.onSurface,
                                    fontWeight: isBest
                                        ? FontWeight.bold
                                        : FontWeight.normal,
                                    fontSize: 11,
                                  ),
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 4),
                          Wrap(
                            spacing: 8,
                            children: [
                              Text(r['hash'] as String,
                                  style: ObsidianTypography.monoCode(
                                      color: ObsidianColors.primary,
                                      fontSize: 9)),
                              ...(r['bonuses'] as List<String>).map((b) => Text(
                                  b,
                                  style: ObsidianTypography.monoCode(
                                      color: ObsidianColors.tertiary,
                                      fontSize: 9))),
                              Text(r['indexer'] as String,
                                  style: ObsidianTypography.monoCode(
                                      color: ObsidianColors.outline,
                                      fontSize: 9)),
                            ],
                          ),
                        ],
                      ),
                    ),

                    // 2. Group
                    Expanded(
                      flex: 2,
                      child: Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 6, vertical: 2),
                        decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainerHighest,
                            borderRadius: BorderRadius.circular(4)),
                        child: Text(r['group'] as String,
                            style: ObsidianTypography.monoCode(
                                color: ObsidianColors.primary, fontSize: 10),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis),
                      ),
                    ),

                    // 3. Format
                    Expanded(
                      flex: 2,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(r['format'] as String,
                              style: ObsidianTypography.monoMetric(
                                  color: ObsidianColors.onSurface,
                                  fontWeight: FontWeight.bold,
                                  fontSize: 10)),
                          Text(r['codec'] as String,
                              style: ObsidianTypography.monoCode(
                                  color: ObsidianColors.outline, fontSize: 9)),
                        ],
                      ),
                    ),

                    // 4. Size
                    Expanded(
                      flex: 2,
                      child: Text(r['size'] as String,
                          textAlign: TextAlign.right,
                          style: ObsidianTypography.monoMetric(
                              color: ObsidianColors.onSurface, fontSize: 10)),
                    ),

                    // 5. Swarm
                    Expanded(
                      flex: 2,
                      child: Text(
                        '${r['seeds']} / ${r['leech']}',
                        textAlign: TextAlign.center,
                        style: ObsidianTypography.monoMetric(
                            color: ObsidianColors.tertiary,
                            fontWeight: FontWeight.bold,
                            fontSize: 10),
                      ),
                    ),

                    // 6. Age
                    Expanded(
                      flex: 1,
                      child: Text(r['age'] as String,
                          style: ObsidianTypography.monoCode(
                              color: ObsidianColors.outline, fontSize: 10)),
                    ),

                    // 7. Score
                    Expanded(
                      flex: 2,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(r['matchScore'] as String,
                              style: ObsidianTypography.monoMetric(
                                  color: isBest
                                      ? ObsidianColors.primary
                                      : ObsidianColors.onSurface,
                                  fontWeight: FontWeight.bold,
                                  fontSize: 10)),
                          Text(r['matchTier'] as String,
                              style: ObsidianTypography.labelBadge(
                                  color: ObsidianColors.outline, fontSize: 8)),
                        ],
                      ),
                    ),

                    // 8. Grab / Queue Button
                    SizedBox(
                      width: 80,
                      child: ElevatedButton.icon(
                        onPressed: () => onGrabRelease(r['title'] as String),
                        icon: const Icon(Icons.download_rounded, size: 12),
                        label: Text(isBest ? 'Grab' : 'Queue',
                            style: ObsidianTypography.titleSm(
                                color: isBest
                                    ? ObsidianColors.onPrimary
                                    : ObsidianColors.onSurface,
                                fontSize: 10)),
                        style: ElevatedButton.styleFrom(
                          backgroundColor: isBest
                              ? ObsidianColors.primary
                              : ObsidianColors.surfaceContainer,
                          foregroundColor: isBest
                              ? ObsidianColors.onPrimary
                              : ObsidianColors.onSurface,
                          elevation: 0,
                          padding: const EdgeInsets.symmetric(
                              horizontal: 8, vertical: 4),
                          shape: RoundedRectangleBorder(
                              borderRadius: BorderRadius.circular(6)),
                        ),
                      ),
                    ),
                  ],
                ),
              );
            },
          ),
        ],
      ),
    );
  }
}
