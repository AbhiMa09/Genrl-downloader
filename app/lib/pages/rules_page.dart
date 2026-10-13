import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';

class RulesPage extends StatelessWidget {
  const RulesPage({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Text('Matching Rules & Preferences', style: ObsidianTypography.headlineLg()),
              const Spacer(),
              ElevatedButton.icon(
                onPressed: () {},
                icon: const Icon(Icons.add_rounded, size: 16),
                label: const Text('Add Show Rule'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.primaryContainer,
                  foregroundColor: ObsidianColors.onPrimaryContainer,
                  elevation: 0,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            'Per-show rules overlay global defaults; custom release group preferences and quality tiers.',
            style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
          ),
          const SizedBox(height: 24),

          ...state.rules.map((rule) {
            return Container(
              margin: const EdgeInsets.only(bottom: 14),
              padding: const EdgeInsets.all(18),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLow,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: ObsidianColors.border),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 6, vertical: 2),
                        decoration: BoxDecoration(
                          color: rule.isGlobal
                              ? ObsidianColors.tertiary.withOpacity(0.15)
                              : ObsidianColors.primary.withOpacity(0.15),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          rule.isGlobal ? 'GLOBAL RULE' : 'SHOW OVERRIDE',
                          style: ObsidianTypography.labelBadge(
                            color: rule.isGlobal
                                ? ObsidianColors.tertiary
                                : ObsidianColors.primary,
                          ),
                        ),
                      ),
                      const SizedBox(width: 10),
                      Text(rule.showTitle, style: ObsidianTypography.titleSm()),
                      const Spacer(),
                      IconButton(
                        onPressed: () {},
                        icon: const Icon(Icons.edit_outlined,
                            size: 16, color: ObsidianColors.outline),
                        padding: EdgeInsets.zero,
                        constraints: const BoxConstraints(),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  Row(
                    children: [
                      _RuleAttribute(
                        label: 'Preferred Groups',
                        value: rule.preferredGroup,
                      ),
                      const SizedBox(width: 24),
                      _RuleAttribute(
                        label: 'Resolution',
                        value: rule.preferredResolution,
                      ),
                      const SizedBox(width: 24),
                      _RuleAttribute(
                        label: 'Min Seeders',
                        value: '${rule.minSeeders}',
                      ),
                      const SizedBox(width: 24),
                      _RuleAttribute(
                        label: 'Score Margin',
                        value: '${rule.scoreMargin.toInt()} pts',
                      ),
                    ],
                  ),
                ],
              ),
            );
          }),
        ],
      ),
    );
  }
}

class _RuleAttribute extends StatelessWidget {
  final String label;
  final String value;

  const _RuleAttribute({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
        const SizedBox(height: 3),
        Text(value, style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface)),
      ],
    );
  }
}
