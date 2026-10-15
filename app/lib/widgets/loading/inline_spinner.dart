import 'dart:math' as math;
import 'package:flutter/material.dart';
import '../../theme/colors.dart';

/// Compact row-level inline spinner for real-time polling and status refresh.
class InlineSpinner extends StatefulWidget {
  final double size;
  final Color color;
  final double strokeWidth;

  const InlineSpinner({
    super.key,
    this.size = 16,
    this.color = ObsidianColors.tertiary,
    this.strokeWidth = 2.0,
  });

  @override
  State<InlineSpinner> createState() => _InlineSpinnerState();
}

class _InlineSpinnerState extends State<InlineSpinner>
    with SingleTickerProviderStateMixin {
  late AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1000),
    )..repeat();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: widget.size,
      height: widget.size,
      child: AnimatedBuilder(
        animation: _controller,
        builder: (context, child) {
          return CustomPaint(
            painter: _InlineSpinnerPainter(
              angle: _controller.value * 2 * math.pi,
              color: widget.color,
              strokeWidth: widget.strokeWidth,
            ),
          );
        },
      ),
    );
  }
}

class _InlineSpinnerPainter extends CustomPainter {
  final double angle;
  final Color color;
  final double strokeWidth;

  _InlineSpinnerPainter({
    required this.angle,
    required this.color,
    required this.strokeWidth,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final radius = (size.width - strokeWidth) / 2;

    // Track circle (faint)
    final trackPaint = Paint()
      ..color = color.withOpacity(0.2)
      ..style = PaintingStyle.stroke
      ..strokeWidth = strokeWidth;
    canvas.drawCircle(center, radius, trackPaint);

    // Active rotating arc
    final arcPaint = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeWidth = strokeWidth;

    canvas.drawArc(
      Rect.fromCircle(center: center, radius: radius),
      angle,
      1.5 * math.pi,
      false,
      arcPaint,
    );
  }

  @override
  bool shouldRepaint(covariant _InlineSpinnerPainter oldDelegate) {
    return oldDelegate.angle != angle ||
        oldDelegate.color != color ||
        oldDelegate.strokeWidth != strokeWidth;
  }
}
