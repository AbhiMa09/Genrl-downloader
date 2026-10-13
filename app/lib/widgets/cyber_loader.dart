import 'dart:math' as math;
import 'package:flutter/material.dart';

/// Cybernetic Ingestion Radar & Packet Receiver Loader.
/// Custom animated canvas rendering:
/// - Outer Technical Tick Track
/// - Outer Dynamic Segment Ring (clockwise spin)
/// - Mid Radar Stream with satellite nodes (counter-clockwise spin)
/// - Inner High-Speed Ring (clockwise spin)
/// - Orbiting Data Nodes with trails
/// - Core Diamond/Hexagonal Packet Receiver with glowing download arrow
class CyberIngestionLoader extends StatefulWidget {
  final double size;
  final bool animate;

  const CyberIngestionLoader({
    super.key,
    this.size = 200,
    this.animate = true,
  });

  @override
  State<CyberIngestionLoader> createState() => _CyberIngestionLoaderState();
}

class _CyberIngestionLoaderState extends State<CyberIngestionLoader>
    with SingleTickerProviderStateMixin {
  late AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 12),
    );
    if (widget.animate) {
      _controller.repeat();
    }
  }

  @override
  void didUpdateWidget(covariant CyberIngestionLoader oldWidget) {
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

    // 1. Background Ambient Glow
    final glowPaint = Paint()
      ..color = const Color(0xFF8B5CF6).withOpacity(0.12)
      ..maskFilter = MaskFilter.blur(BlurStyle.normal, 28 * scale);
    canvas.drawCircle(center, 70 * scale, glowPaint);

    // 2. Outer Technical Tick Track (Radius ~102)
    final tickTrackPaint = Paint()
      ..color = const Color(0xFF252B36).withOpacity(0.8)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2 * scale;
    _drawDashedCircle(canvas, center, 102 * scale, tickTrackPaint, dash: 3 * scale, gap: 8 * scale);

    // 3. Outer Dynamic Segment Ring (Radius ~95, spins clockwise)
    final outerAngle = progress * 2 * math.pi * 2.0; // 6s per loop
    canvas.save();
    canvas.translate(center.dx, center.dy);
    canvas.rotate(outerAngle);

    final outerArcPaint = Paint()
      ..shader = const SweepGradient(
        colors: [
          Color(0xFFA78BFA),
          Color(0xFF8B5CF6),
          Color(0x336366F1),
          Color(0xFFA78BFA),
        ],
      ).createShader(Rect.fromCircle(center: Offset.zero, radius: 95 * scale))
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeWidth = 3.2 * scale;
    canvas.drawArc(
      Rect.fromCircle(center: Offset.zero, radius: 95 * scale),
      0,
      math.pi * 1.1,
      false,
      outerArcPaint,
    );

    // Accent mini tick on outer ring
    final outerTickPaint = Paint()
      ..color = const Color(0xFFA78BFA)
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeWidth = 4 * scale;
    canvas.drawArc(
      Rect.fromCircle(center: Offset.zero, radius: 95 * scale),
      math.pi * 1.35,
      0.35,
      false,
      outerTickPaint,
    );
    canvas.restore();

    // 4. Mid Radar Stream (Radius ~76, spins counter-clockwise)
    final midAngle = -progress * 2 * math.pi * 3.0; // 4s per loop
    canvas.save();
    canvas.translate(center.dx, center.dy);
    canvas.rotate(midAngle);

    final midArcPaint = Paint()
      ..shader = const SweepGradient(
        colors: [
          Color(0xFF06B6D4),
          Color(0xFF8B5CF6),
          Color(0x108B5CF6),
          Color(0xFF06B6D4),
        ],
      ).createShader(Rect.fromCircle(center: Offset.zero, radius: 76 * scale))
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeWidth = 3.5 * scale;

    canvas.drawArc(
      Rect.fromCircle(center: Offset.zero, radius: 76 * scale),
      0,
      math.pi * 0.9,
      false,
      midArcPaint,
    );

    canvas.drawArc(
      Rect.fromCircle(center: Offset.zero, radius: 76 * scale),
      math.pi * 1.2,
      math.pi * 0.4,
      false,
      midArcPaint,
    );

    // Satellites on mid ring
    final satPaintCyan = Paint()..color = const Color(0xFF06B6D4);
    canvas.drawCircle(Offset(0, -76 * scale), 4 * scale, satPaintCyan);

    final satPaintViolet = Paint()..color = const Color(0xFF8B5CF6);
    canvas.drawCircle(Offset(0, 76 * scale), 3 * scale, satPaintViolet);

    canvas.restore();

    // 5. Inner High-Speed Ring (Radius ~54, spins clockwise fast)
    final innerAngle = progress * 2 * math.pi * 5.4; // ~2.2s per loop
    canvas.save();
    canvas.translate(center.dx, center.dy);
    canvas.rotate(innerAngle);

    final innerArcPaint = Paint()
      ..color = const Color(0xFFC4B5FD).withOpacity(0.85)
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeWidth = 2.5 * scale;

    _drawDashedCircle(canvas, Offset.zero, 54 * scale, innerArcPaint, dash: 24 * scale, gap: 16 * scale);
    canvas.restore();

    // 6. Orbiting Download Data Nodes (Radius ~92 and ~68)
    final orbAngle1 = progress * 2 * math.pi * 4.0;
    final orb1 = Offset(
      center.dx + math.cos(orbAngle1) * 92 * scale,
      center.dy + math.sin(orbAngle1) * 92 * scale,
    );
    final orbGlow1 = Paint()
      ..color = const Color(0xFFA78BFA)
      ..maskFilter = MaskFilter.blur(BlurStyle.normal, 5 * scale);
    canvas.drawCircle(orb1, 4 * scale, orbGlow1);
    canvas.drawCircle(orb1, 3.5 * scale, Paint()..color = const Color(0xFFA78BFA));

    final orbAngle2 = -progress * 2 * math.pi * 4.8;
    final orb2 = Offset(
      center.dx + math.cos(orbAngle2) * 62 * scale,
      center.dy + math.sin(orbAngle2) * 62 * scale,
    );
    canvas.drawCircle(orb2, 3 * scale, Paint()..color = const Color(0xFF38BDF8));

    // 7. Core Node with Diamond / Rounded Rect Background & Pulsing Core
    final pulseScale = 0.95 + 0.1 * math.sin(progress * 2 * math.pi * 5.0);
    canvas.save();
    canvas.translate(center.dx, center.dy);
    canvas.scale(pulseScale, pulseScale);

    // Rotated rounded diamond (45 deg)
    canvas.save();
    canvas.rotate(math.pi / 4);
    final coreRect = Rect.fromCenter(center: Offset.zero, width: 44 * scale, height: 44 * scale);
    final coreRRect = RRect.fromRectAndRadius(coreRect, Radius.circular(12 * scale));

    final coreFill = Paint()
      ..color = const Color(0xFF181C24)
      ..style = PaintingStyle.fill;
    canvas.drawRRect(coreRRect, coreFill);

    final coreStroke = Paint()
      ..color = const Color(0xFF8B5CF6)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2 * scale;
    canvas.drawRRect(coreRRect, coreStroke);
    canvas.restore();

    // Download Arrow & Base Line
    final arrowPaint = Paint()
      ..color = const Color(0xFFF1F5F9)
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round
      ..strokeWidth = 3 * scale;

    // Vertical shaft
    canvas.drawLine(Offset(0, -12 * scale), Offset(0, 7 * scale), arrowPaint);
    // Arrow head
    final headPath = Path()
      ..moveTo(-6 * scale, 1 * scale)
      ..lineTo(0, 8 * scale)
      ..lineTo(6 * scale, 1 * scale);
    canvas.drawPath(headPath, arrowPaint);

    // Ingestion Receiver Tray / Line below
    final trayPaint = Paint()
      ..color = const Color(0xFF8B5CF6)
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeWidth = 2.5 * scale;
    canvas.drawLine(Offset(-9 * scale, 13 * scale), Offset(9 * scale, 13 * scale), trayPaint);

    canvas.restore();

    // 8. Hexagon Data Corner Dots
    final dotPaint = Paint()..color = const Color(0xFFA78BFA).withOpacity(0.6);
    final dOffset = 26 * scale;
    canvas.drawCircle(Offset(center.dx - dOffset, center.dy - dOffset), 1.5 * scale, dotPaint);
    canvas.drawCircle(Offset(center.dx + dOffset, center.dy - dOffset), 1.5 * scale, dotPaint);
    canvas.drawCircle(Offset(center.dx - dOffset, center.dy + dOffset), 1.5 * scale, dotPaint);
    canvas.drawCircle(Offset(center.dx + dOffset, center.dy + dOffset), 1.5 * scale, dotPaint);
  }

  void _drawDashedCircle(Canvas canvas, Offset center, double radius, Paint paint,
      {required double dash, required double gap}) {
    final circumference = 2 * math.pi * radius;
    final totalSteps = (circumference / (dash + gap)).floor();
    final actualDashAngle = (dash / circumference) * 2 * math.pi;
    final stepAngle = (2 * math.pi) / totalSteps;

    for (int i = 0; i < totalSteps; i++) {
      final start = i * stepAngle;
      canvas.drawArc(
        Rect.fromCircle(center: center, radius: radius),
        start,
        actualDashAngle,
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
