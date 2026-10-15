import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class BitrateSparklineCard extends StatelessWidget {
  final String durationLabel;

  const BitrateSparklineCard({
    super.key,
    required this.durationLabel,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Flexible(
                child: Text(
                  'BITRATE PROFILE ($durationLabel)',
                  style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 9),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 4),
              Text(
                'Avg 18.7 Mbps',
                style: ObsidianTypography.monoMetric(color: ObsidianColors.tertiary).copyWith(fontSize: 10),
              ),
            ],
          ),
          const SizedBox(height: 10),

          // Custom Painted Sparkline Chart
          Container(
            height: 48,
            width: double.infinity,
            padding: const EdgeInsets.all(4),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: CustomPaint(
              painter: _BitrateSparklinePainter(),
            ),
          ),
          const SizedBox(height: 6),

          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('00:00', style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 9)),
              Text('Bitrate Sparkline', style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 9)),
              Text(durationLabel,
                  style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 9)),
            ],
          ),
        ],
      ),
    );
  }
}

class _BitrateSparklinePainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final w = size.width;
    final h = size.height;

    final path = Path();
    // Normalized curve points matching the SVG path in code.html
    final points = [
      Offset(0.0, h * 0.8),
      Offset(w * 0.1, h * 0.45),
      Offset(w * 0.21, h * 0.62),
      Offset(w * 0.32, h * 0.2),
      Offset(w * 0.43, h * 0.5),
      Offset(w * 0.54, h * 0.35),
      Offset(w * 0.64, h * 0.15),
      Offset(w * 0.75, h * 0.3),
      Offset(w * 0.86, h * 0.6),
      Offset(w * 0.96, h * 0.25),
      Offset(w, h * 0.45),
    ];

    path.moveTo(points[0].dx, points[0].dy);
    for (int i = 0; i < points.length - 1; i++) {
      final p0 = points[i];
      final p1 = points[i + 1];
      final controlX = (p0.dx + p1.dx) / 2;
      path.cubicTo(controlX, p0.dy, controlX, p1.dy, p1.dx, p1.dy);
    }

    // Fill area under curve with vertical gradient
    final fillPath = Path.from(path)
      ..lineTo(w, h)
      ..lineTo(0, h)
      ..close();

    final fillPaint = Paint()
      ..shader = LinearGradient(
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
        colors: [
          ObsidianColors.primaryContainer.withOpacity(0.45),
          ObsidianColors.primaryContainer.withOpacity(0.0),
        ],
      ).createShader(Rect.fromLTWH(0, 0, w, h));
    canvas.drawPath(fillPath, fillPaint);

    // Stroke curve with primary violet
    final strokePaint = Paint()
      ..color = ObsidianColors.primary
      ..strokeWidth = 1.75
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round;
    canvas.drawPath(path, strokePaint);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
