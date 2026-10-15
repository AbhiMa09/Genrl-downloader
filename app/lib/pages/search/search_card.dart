import 'package:flutter/material.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class SearchCard extends StatelessWidget {
  final StateProvider state;
  final TextEditingController searchController;
  final bool isSearching;
  final VoidCallback onSearch;
  final VoidCallback onReset;

  const SearchCard({
    super.key,
    required this.state,
    required this.searchController,
    required this.isSearching,
    required this.onSearch,
    required this.onReset,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          LayoutBuilder(
            builder: (context, constraints) {
              final isStacked = constraints.maxWidth < 800;

              return Flex(
                direction: isStacked ? Axis.vertical : Axis.horizontal,
                children: [
                  // Main Query Input
                  Expanded(
                    flex: isStacked ? 0 : 8,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            Text('QUERY STRING / TORZNAB QUERY',
                                style: ObsidianTypography.labelBadge(
                                    color: ObsidianColors.outline,
                                    fontSize: 9)),
                            Text('Auto-Sanitizing CRC32 hashes',
                                style: ObsidianTypography.monoCode(
                                    color: ObsidianColors.outline,
                                    fontSize: 9)),
                          ],
                        ),
                        const SizedBox(height: 6),
                        Container(
                          height: 44,
                          padding: const EdgeInsets.symmetric(horizontal: 10),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainerLowest,
                            borderRadius: BorderRadius.circular(8),
                            border: Border.all(color: ObsidianColors.border),
                          ),
                          child: Row(
                            children: [
                              const Icon(Icons.search_rounded,
                                  size: 20, color: ObsidianColors.outline),
                              const SizedBox(width: 8),
                              Expanded(
                                child: TextField(
                                  controller: searchController,
                                  style: ObsidianTypography.monoCode(
                                      color: ObsidianColors.onSurface,
                                      fontSize: 12),
                                  decoration: InputDecoration(
                                    hintText:
                                        'Search anime release or torrent hash...',
                                    hintStyle: ObsidianTypography.monoCode(
                                        color: ObsidianColors.outline,
                                        fontSize: 12),
                                    border: InputBorder.none,
                                    isDense: true,
                                  ),
                                  onSubmitted: (_) => onSearch(),
                                ),
                              ),
                              IconButton(
                                icon: const Icon(Icons.close_rounded,
                                    size: 16, color: ObsidianColors.outline),
                                onPressed: onReset,
                                padding: EdgeInsets.zero,
                                constraints: const BoxConstraints(),
                              ),
                              const SizedBox(width: 6),
                              Container(
                                padding: const EdgeInsets.symmetric(
                                    horizontal: 6, vertical: 2),
                                decoration: BoxDecoration(
                                  color:
                                      ObsidianColors.surfaceContainerHighest,
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text('ENTER',
                                    style: ObsidianTypography.monoCode(
                                        color: ObsidianColors.outline,
                                        fontSize: 9)),
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                  if (!isStacked)
                    const SizedBox(width: 14)
                  else
                    const SizedBox(height: 14),

                  // Target Binding Dropdown / Preview
                  Expanded(
                    flex: isStacked ? 0 : 4,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            Text('LIBRARY TARGET BINDING',
                                style: ObsidianTypography.labelBadge(
                                    color: ObsidianColors.outline,
                                    fontSize: 9)),
                            Text('Linked',
                                style: ObsidianTypography.monoCode(
                                    color: ObsidianColors.tertiary,
                                    fontSize: 9)),
                          ],
                        ),
                        const SizedBox(height: 6),
                        Container(
                          height: 44,
                          padding: const EdgeInsets.symmetric(horizontal: 8),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainerLowest,
                            borderRadius: BorderRadius.circular(8),
                            border: Border.all(color: ObsidianColors.border),
                          ),
                          child: Row(
                            children: [
                              Container(
                                width: 28,
                                height: 32,
                                decoration: BoxDecoration(
                                  color: ObsidianColors
                                      .surfaceContainerHighest,
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                clipBehavior: Clip.antiAlias,
                                child: (state.selectedShow?.posterUrl ??
                                            (state.shows.isNotEmpty
                                                ? state.shows.first.posterUrl
                                                : null)) !=
                                        null
                                    ? Image.network(
                                        (state.selectedShow?.posterUrl ??
                                            state.shows.first.posterUrl)!,
                                        fit: BoxFit.cover,
                                        errorBuilder: (_, __, ___) =>
                                            const Icon(Icons.movie_rounded,
                                                size: 14,
                                                color: ObsidianColors.outline),
                                      )
                                    : const Icon(Icons.movie_rounded,
                                        size: 14,
                                        color: ObsidianColors.outline),
                              ),
                              const SizedBox(width: 8),
                              Expanded(
                                child: Column(
                                  crossAxisAlignment:
                                      CrossAxisAlignment.start,
                                  mainAxisAlignment:
                                      MainAxisAlignment.center,
                                  children: [
                                    Text(
                                      state.selectedShow?.title ??
                                          (state.shows.isNotEmpty
                                              ? state.shows.first.title
                                              : 'No Target Bound'),
                                      style: ObsidianTypography.titleSm(
                                          fontSize: 11),
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                    Text(
                                      state.shows.isNotEmpty
                                          ? 'S01 • Monitored'
                                          : 'Manual Indexer Search',
                                      style: ObsidianTypography.monoCode(
                                          color: ObsidianColors.outline,
                                          fontSize: 9),
                                    ),
                                  ],
                                ),
                              ),
                              const Icon(Icons.unfold_more_rounded,
                                  size: 16, color: ObsidianColors.outline),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              );
            },
          ),
          const SizedBox(height: 12),

          // Action Button Strip
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Row(
                children: [
                  Container(
                      width: 6,
                      height: 6,
                      decoration: const BoxDecoration(
                          color: ObsidianColors.primary,
                          shape: BoxShape.circle)),
                  const SizedBox(width: 6),
                  Text('Auto-categorized as: Anime / 1080p Web-DL',
                      style: ObsidianTypography.monoCode(
                          color: ObsidianColors.outline, fontSize: 10)),
                ],
              ),
              Row(
                children: [
                  OutlinedButton.icon(
                    onPressed: onReset,
                    icon: const Icon(Icons.restart_alt_rounded, size: 14),
                    label: Text('Reset Query',
                        style: ObsidianTypography.bodySm(fontSize: 11)),
                    style: OutlinedButton.styleFrom(
                      side: const BorderSide(color: ObsidianColors.border),
                      padding: const EdgeInsets.symmetric(
                          horizontal: 10, vertical: 6),
                    ),
                  ),
                  const SizedBox(width: 8),
                  ElevatedButton.icon(
                    onPressed: isSearching ? null : onSearch,
                    icon: isSearching
                        ? const SizedBox(
                            width: 14,
                            height: 14,
                            child: CircularProgressIndicator(
                                strokeWidth: 2,
                                color: ObsidianColors.onPrimary))
                        : const Icon(Icons.travel_explore_rounded, size: 16),
                    label: Text('Search Indexers',
                        style: ObsidianTypography.titleSm(
                            color: ObsidianColors.onPrimary, fontSize: 11)),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: ObsidianColors.primary,
                      foregroundColor: ObsidianColors.onPrimary,
                      elevation: 0,
                      padding: const EdgeInsets.symmetric(
                          horizontal: 14, vertical: 8),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
    );
  }
}
