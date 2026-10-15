import 'package:flutter/material.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class SettingsCommandStrip extends StatelessWidget {
  final StateProvider state;
  final VoidCallback onSave;

  const SettingsCommandStrip({
    super.key,
    required this.state,
    required this.onSave,
  });

  @override
  Widget build(BuildContext context) {
    return Wrap(
      alignment: WrapAlignment.spaceBetween,
      crossAxisAlignment: WrapCrossAlignment.center,
      spacing: 16,
      runSpacing: 12,
      children: [
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Wrap(
              crossAxisAlignment: WrapCrossAlignment.center,
              spacing: 6,
              children: [
                Text('System / Configuration / ', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1.5),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHigh,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text('SESSION ACTIVE', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Wrap(
              crossAxisAlignment: WrapCrossAlignment.center,
              spacing: 8,
              children: [
                Text('Settings & System Integration', style: ObsidianTypography.headlineLg()),
                Text('v2.4.1', style: ObsidianTypography.titleSm(color: ObsidianColors.outline)),
              ],
            ),
          ],
        ),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            OutlinedButton.icon(
              onPressed: () {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('Preferences reloaded from disk')),
                );
              },
              icon: const Icon(Icons.restart_alt_rounded, size: 16),
              label: const Text('Discard Changes'),
              style: OutlinedButton.styleFrom(
                side: const BorderSide(color: ObsidianColors.border),
                foregroundColor: ObsidianColors.onSurfaceVariant,
              ),
            ),
            ElevatedButton.icon(
              onPressed: onSave,
              icon: const Icon(Icons.check_rounded, size: 16),
              label: const Text('Save Preferences'),
              style: ElevatedButton.styleFrom(
                backgroundColor: ObsidianColors.primaryContainer,
                foregroundColor: ObsidianColors.onPrimaryContainer,
              ),
            ),
          ],
        ),
      ],
    );
  }
}
