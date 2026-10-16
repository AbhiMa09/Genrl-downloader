import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../theme/colors.dart';

/// Concentric rings cyber loader matching the Stitch design tokens.
/// Features an outer dashed ring, middle radar stream with satellite dots,
/// inner high-speed ring, and a pulsing core download emblem.
class CyberLoader extends StatefulWidget {
  final double size;
  final bool isAnimating;

  const CyberLoader({super.key, this.size = 200, this.isAnimating = true});

  @override
  State<CyberLoader> createState() => _CyberLoaderState();
}

class _CyberLoaderState extends State<CyberLoader>
    with SingleTickerProviderStateMixin {
  late AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 12),
    );
    if (widget.isAnimating) {
      _controller.repeat();
    }
  }

  @override
  void didUpdateWidget(CyberLoader oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.isAnimating != oldWidget.isAnimating) {
      if (widget.isAnimating) {
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
    return AnimatedBuilder(
      animation: _controller,
      builder: (context, child) {
        return CustomPaint(
          size: Size(widget.size, widget.size),
          painter: _CyberLoaderPainter(progress: _controller.value),
        );
      },
    );
  }
}

class _CyberLoaderPainter extends CustomPainter {
  final double progress;

  _CyberLoaderPainter({required this.progress});

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final scale = size.width / 240.0;

    // 1. Outer Technical Tick Track
    final tickPaint = Paint()
      ..color = const Color(0xFF252B36).withOpacity(0.6)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2 * scale;
    _drawDashedCircle(canvas, center, 102 * scale, 36, tickPaint);

    // 2. Outer Dynamic Segment Ring (clockwise, 6s per rotation)
    final outerAngle = progress * 2 * math.pi * 2;
    canvas.save();
    canvas.translate(center.dx, center.dy);
    canvas.rotate(outerAngle);
    final outerPaint = Paint()
      ..color = ObsidianColors.primary
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeWidth = 3 * scale;
    canvas.drawArc(
      Rect.fromCircle(center: Offset.zero, radius: 95 * scale),
      0,
      1.8 * math.pi,
      false,
      outerPaint,
    );
    canvas.restore();

    // 3. Middle Radar Stream (counter-clockwise, 4s per rotation)
    final midAngle = -progress * 2 * math.pi * 3;
    canvas.save();
    canvas.translate(center.dx, center.dy);
    canvas.rotate(midAngle);
    final midPaint = Paint()
      ..color = ObsidianColors.tertiary
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeWidth = 3.5 * scale;
    canvas.drawArc(
      Rect.fromCircle(center: Offset.zero, radius: 76 * scale),
      0,
      1.2 * math.pi,
      false,
      midPaint,
    );
    // Satellite dots
    final sat1 = Paint()..color = const Color(0xFF06B6D4);
    canvas.drawCircle(Offset(0, -76 * scale), 3.5 * scale, sat1);
    final sat2 = Paint()..color = ObsidianColors.primary;
    canvas.drawCircle(Offset(0, 76 * scale), 2.5 * scale, sat2);
    canvas.restore();

    // 4. Inner High-Speed Ring (clockwise, 2.2s per rotation)
    final innerAngle = progress * 2 * math.pi * 5.4;
    canvas.save();
    canvas.translate(center.dx, center.dy);
    canvas.rotate(innerAngle);
    final innerPaint = Paint()
      ..color = const Color(0xFFC4B5FD).withOpacity(0.85)
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeWidth = 2.5 * scale;
    canvas.drawArc(
      Rect.fromCircle(center: Offset.zero, radius: 54 * scale),
      0,
      math.pi,
      false,
      innerPaint,
    );
    canvas.restore();

    // 5. Core Node: Diamond background & download arrow
    final pulse = 0.95 + 0.08 * math.sin(progress * 2 * math.pi * 5);
    canvas.save();
    canvas.translate(center.dx, center.dy);
    canvas.scale(scale * pulse);

    final diamondRect = RRect.fromRectAndRadius(
      Rect.fromCenter(center: Offset.zero, width: 44, height: 44),
      const Radius.circular(12),
    );
    canvas.save();
    canvas.rotate(math.pi / 4);
    final coreBg = Paint()..color = const Color(0xFF181C24);
    final coreBorder = Paint()
      ..color = ObsidianColors.primary
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2;
    canvas.drawRRect(diamondRect, coreBg);
    canvas.drawRRect(diamondRect, coreBorder);
    canvas.restore();

    // Central download arrow
    final arrowPaint = Paint()
      ..color = const Color(0xFFF1F5F9)
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round
      ..strokeWidth = 3;
    final arrowPath = Path()
      ..moveTo(0, -14)
      ..lineTo(0, 7)
      ..moveTo(-7, 1)
      ..lineTo(0, 8)
      ..lineTo(7, 1);
    canvas.drawPath(arrowPath, arrowPaint);

    final linePaint = Paint()
      ..color = ObsidianColors.primary
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeWidth = 2.5;
    canvas.drawLine(const Offset(-10, 14), const Offset(10, 14), linePaint);

    canvas.restore();
  }

  void _drawDashedCircle(
    Canvas canvas,
    Offset center,
    double radius,
    int dashes,
    Paint paint,
  ) {
    const sweep = 2 * math.pi;
    final dashAngle = sweep / dashes;
    for (int i = 0; i < dashes; i++) {
      final start = i * dashAngle;
      canvas.drawArc(
        Rect.fromCircle(center: center, radius: radius),
        start,
        dashAngle * 0.4,
        false,
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(covariant _CyberLoaderPainter oldDelegate) {
    return oldDelegate.progress != progress;
  }
}
