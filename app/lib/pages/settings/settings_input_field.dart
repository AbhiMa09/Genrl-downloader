import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class SettingsInputField extends StatelessWidget {
  final String label;
  final String? sublabel;
  final IconData icon;
  final TextEditingController controller;

  const SettingsInputField({
    super.key,
    required this.label,
    this.sublabel,
    required this.icon,
    required this.controller,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Expanded(
              child: Text(label, style: ObsidianTypography.monoCode(), overflow: TextOverflow.ellipsis),
            ),
            if (sublabel != null) ...[
              const SizedBox(width: 8),
              Text(sublabel!, style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
            ],
          ],
        ),
        const SizedBox(height: 4),
        Container(
          height: 38,
          padding: const EdgeInsets.symmetric(horizontal: 10),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: ObsidianColors.border),
          ),
          child: Row(
            children: [
              Icon(icon, size: 16, color: ObsidianColors.outline),
              const SizedBox(width: 8),
              Expanded(
                child: TextField(
                  controller: controller,
                  style: ObsidianTypography.monoCode(),
                  decoration: const InputDecoration(border: InputBorder.none, isDense: true, contentPadding: EdgeInsets.zero),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}
