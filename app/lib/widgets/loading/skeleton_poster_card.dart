import 'package:flutter/material.dart';
import '../../theme/colors.dart';

/// 2:3 aspect ratio skeleton card with gradient shimmer sweep.
/// Used in Library and Show Detail views while remote anime posters are being resolved.
class SkeletonPosterCard extends StatefulWidget {
  final double? width;
  final double? height;
  final double aspectRatio;
  final double borderRadius;

  const SkeletonPosterCard({
    super.key,
    this.width,
    this.height,
    this.aspectRatio = 2 / 3,
    this.borderRadius = 12.0,
  });

  @override
  State<SkeletonPosterCard> createState() => _SkeletonPosterCardState();
}

class _SkeletonPosterCardState extends State<SkeletonPosterCard>
    with SingleTickerProviderStateMixin {
  late AnimationController _shimmerController;

  @override
  void initState() {
    super.initState();
    _shimmerController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1800),
    )..repeat();
  }

  @override
  void dispose() {
    _shimmerController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    Widget cardBody = ClipRRect(
      borderRadius: BorderRadius.circular(widget.borderRadius),
      child: AnimatedBuilder(
        animation: _shimmerController,
        builder: (context, child) {
          final shimmerValue = _shimmerController.value;
          return Container(
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerHigh,
              border: Border.all(
                color: ObsidianColors.border.withOpacity(0.5),
                width: 0.5,
              ),
              gradient: LinearGradient(
                begin: Alignment(-2.0 + shimmerValue * 4.0, -0.3),
                end: Alignment(-1.0 + shimmerValue * 4.0, 0.3),
                colors: [
                  ObsidianColors.surfaceContainerHigh,
                  ObsidianColors.surfaceContainerHighest.withOpacity(0.6),
                  ObsidianColors.surfaceContainerHigh,
                ],
              ),
            ),
            padding: const EdgeInsets.all(12),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                // Top placeholder badge
                const Spacer(),
                // Title skeleton bar
                Container(
                  height: 12,
                  width: double.infinity,
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLowest.withOpacity(0.5),
                    borderRadius: BorderRadius.circular(4),
                  ),
                ),
                const SizedBox(height: 6),
                // Subtitle skeleton bar
                Container(
                  height: 10,
                  width: 70,
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLowest.withOpacity(0.4),
                    borderRadius: BorderRadius.circular(4),
                  ),
                ),
              ],
            ),
          );
        },
      ),
    );

    if (widget.width != null || widget.height != null) {
      return SizedBox(
        width: widget.width,
        height: widget.height,
        child: cardBody,
      );
    }

    return AspectRatio(
      aspectRatio: widget.aspectRatio,
      child: cardBody,
    );
  }
}
