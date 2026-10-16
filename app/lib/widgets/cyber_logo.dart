import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../theme/colors.dart';

/// Genrl Downloader Cybernetic Emblem & Official Logo.
/// Supports both high-res asset rendering and a live-reactive canvas mode:
/// - Outer Squircle with Obsidian gradient & glowing border
/// - Stylized "G" Loop & Orbit with Neon Violet Glow
/// - Central Precision Downloader Chevron that dynamically pulses with transfer speeds
/// - Receiving Platform with glowing cyan indicator
/// - Speed arcs and orbiting satellites
class CyberLogo extends StatefulWidget {
  final double size;
  final bool animate;
  final double speedBytes;
  final bool showSquircle;

  const CyberLogo({
    super.key,
    this.size = 32,
    this.animate = true,
    this.speedBytes = 0,
    this.showSquircle = true,
  });

  @override
  State<CyberLogo> createState() => _CyberLogoState();
}

class _CyberLogoState extends State<CyberLogo>
    with SingleTickerProviderStateMixin {
  late AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 4),
    );
    if (widget.animate) {
      _controller.repeat();
    }
  }

  @override
  void didUpdateWidget(covariant CyberLogo oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.animate != oldWidget.animate) {
      if (widget.animate) {
        _controller.repeat();
      } else {
        _controller.stop();
      }
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final double s = widget.size;

    return SizedBox(
      width: s,
      height: s,
      child: Stack(
        alignment: Alignment.center,
        children: [
          // High-res raster/vector asset container
          ClipRRect(
            borderRadius: BorderRadius.circular(s * (104.0 / 512.0)),
            child: Image.asset(
              'assets/logo.png',
              width: s,
              height: s,
              fit: BoxFit.cover,
              errorBuilder: (_, __, ___) => _canvasFallback(s),
            ),
          ),

          // Live reactive pulse overlay when transfers are active
          if (widget.animate && widget.speedBytes > 0)
            AnimatedBuilder(
              animation: _controller,
              builder: (context, _) {
                final pulse =
                    0.5 + 0.5 * math.sin(_controller.value * 2 * math.pi);
                return Container(
                  width: s,
                  height: s,
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(s * (104.0 / 512.0)),
                    border: Border.all(
                      color: ObsidianColors.tertiary.withOpacity(0.3 * pulse),
                      width: 1.5,
                    ),
                    boxShadow: [
                      BoxShadow(
                        color: ObsidianColors.primary.withOpacity(0.25 * pulse),
                        blurRadius: 10 * pulse,
                        spreadRadius: 2 * pulse,
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

  Widget _canvasFallback(double size) {
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(size * 0.2),
        border: Border.all(color: ObsidianColors.primary.withOpacity(0.4)),
      ),
      child: Center(
        child: Icon(
          Icons.download_rounded,
          size: size * 0.5,
          color: ObsidianColors.primary,
        ),
      ),
    );
  }
}
