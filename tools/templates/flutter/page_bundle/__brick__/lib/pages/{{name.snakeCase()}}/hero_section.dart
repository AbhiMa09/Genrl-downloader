import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../theme/spacing.dart';

/// Hero header section widget for {{name.pascalCase()}}.
class {{name.pascalCase()}}HeroSection extends StatelessWidget {
  final String title;

  const {{name.pascalCase()}}HeroSection({
    super.key,
    required this.title,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(ObsidianSpacing.spaceMd),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(ObsidianRadius.md),
        border: Border.all(color: ObsidianColors.outlineVariant),
      ),
      child: Row(
        children: [
          Icon(Icons.dashboard_customize, color: ObsidianColors.primary, size: 28),
          const SizedBox(width: ObsidianSpacing.spaceSm),
          Text(
            title,
            style: ObsidianTypography.headlineLg(),
          ),
        ],
      ),
    );
  }
}
