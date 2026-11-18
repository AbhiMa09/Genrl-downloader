import 'package:flutter/material.dart';

import '../models/media_item.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import 'cyber_badge.dart';
import 'loading/skeleton_poster_card.dart';

/// Reusable virtualized 2:3 poster grid that matches the Library page styling.
class PosterGrid extends StatelessWidget {
  final List<MediaListItem> items;
  final ValueChanged<MediaListItem> onItemTap;
  final ScrollController? scrollController;
  final EdgeInsetsGeometry padding;

  const PosterGrid({
    super.key,
    required this.items,
    required this.onItemTap,
    this.scrollController,
    this.padding = const EdgeInsets.all(16),
  });

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth;
        // Determine column count matching Library density
        final crossAxisCount = (width / 180).floor().clamp(2, 8);

        return GridView.builder(
          controller: scrollController,
          padding: padding,
          gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
            crossAxisCount: crossAxisCount,
            crossAxisSpacing: 16,
            mainAxisSpacing: 16,
            childAspectRatio: 2 / 3.8,
          ),
          itemCount: items.length,
          itemBuilder: (context, index) {
            final item = items[index];
            return PosterGridCard(item: item, onTap: () => onItemTap(item));
          },
        );
      },
    );
  }
}

class PosterGridCard extends StatefulWidget {
  final MediaListItem item;
  final VoidCallback onTap;

  const PosterGridCard({super.key, required this.item, required this.onTap});

  @override
  State<PosterGridCard> createState() => _PosterGridCardState();
}

class _PosterGridCardState extends State<PosterGridCard> {
  bool _isHovered = false;

  Widget _buildPoster(String? url) {
    if (url != null && url.isNotEmpty) {
      return Image.network(
        url,
        fit: BoxFit.cover,
        cacheWidth: 342,
        loadingBuilder: (context, child, progress) {
          if (progress == null) return child;
          return const SkeletonPosterCard();
        },
        errorBuilder: (_, __, ___) => _buildPlaceholder(),
      );
    }
    return _buildPlaceholder();
  }

  Widget _buildPlaceholder() {
    return Container(
      color: ObsidianColors.surfaceContainerHigh,
      child: Center(
        child: Icon(
          widget.item.kind == 'movie'
              ? Icons.movie_outlined
              : Icons.tv_outlined,
          size: 40,
          color: ObsidianColors.outlineVariant,
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final item = widget.item;

    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: InkWell(
        onTap: widget.onTap,
        borderRadius: BorderRadius.circular(12),
        child: Container(
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainerLow,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(
              color: _isHovered
                  ? ObsidianColors.primary.withOpacity(0.6)
                  : ObsidianColors.border,
            ),
            boxShadow: _isHovered
                ? [
                    BoxShadow(
                      color: ObsidianColors.primary.withOpacity(0.2),
                      blurRadius: 16,
                      offset: const Offset(0, 4),
                    ),
                  ]
                : null,
          ),
          clipBehavior: Clip.antiAlias,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // 2:3 AspectRatio Poster
              AspectRatio(
                aspectRatio: 2 / 3,
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    _buildPoster(item.posterUrl),
                    // Status Badge in top right corner
                    Positioned(
                      top: 8,
                      right: 8,
                      child: CyberBadge.fromStatus(item.status),
                    ),
                  ],
                ),
              ),

              // Metadata strip
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 8,
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text(
                        item.title,
                        style: ObsidianTypography.titleSm(),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          if (item.year != null)
                            Text(
                              item.year.toString(),
                              style: ObsidianTypography.bodySm(
                                color: ObsidianColors.outline,
                              ),
                            )
                          else
                            const SizedBox.shrink(),
                          if (item.kind == 'series' && item.progress != null)
                            Text(
                              item.progress!.formatSummary(),
                              style: ObsidianTypography.monoCode(
                                color: ObsidianColors.primary,
                              ),
                            ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
