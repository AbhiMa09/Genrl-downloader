import 'dart:io';

import 'package:flutter/material.dart';

import '../theme/colors.dart';

import 'loading/skeleton_poster_card.dart';

class AnimePoster extends StatelessWidget {
  final String? urlOrPath;
  final double aspectRatio;
  final BorderRadius? borderRadius;

  const AnimePoster({
    super.key,
    this.urlOrPath,
    this.aspectRatio = 2 / 3,
    this.borderRadius,
  });

  @override
  Widget build(BuildContext context) {
    final radius = borderRadius ?? BorderRadius.circular(8);
    final target = urlOrPath;

    Widget content;
    if (target != null && target.isNotEmpty) {
      if (target.startsWith('http')) {
        content = Image.network(
          target,
          fit: BoxFit.cover,
          cacheWidth: 320,
          cacheHeight: 480,
          loadingBuilder: (context, child, progress) {
            if (progress == null) return child;
            return const SkeletonPosterCard();
          },
          errorBuilder: (_, __, ___) => _placeholder(),
        );
      } else {
        content = Image.file(
          File(target),
          fit: BoxFit.cover,
          cacheWidth: 320,
          cacheHeight: 480,
          errorBuilder: (_, __, ___) => _placeholder(),
        );
      }
    } else {
      content = _placeholder();
    }

    return AspectRatio(
      aspectRatio: aspectRatio,
      child: ClipRRect(
        borderRadius: radius,
        child: Container(
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainerHigh,
            border: Border.all(color: ObsidianColors.border, width: 0.5),
          ),
          child: content,
        ),
      ),
    );
  }

  Widget _placeholder() {
    return Container(
      color: ObsidianColors.surfaceContainerHigh,
      child: const Center(
        child: Icon(
          Icons.movie_outlined,
          size: 28,
          color: ObsidianColors.outline,
        ),
      ),
    );
  }
}
