import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class RulesHeaderScope extends StatelessWidget {
  final StateProvider state;
  final String selectedScope;
  final ValueChanged<String> onScopeChanged;
  final VoidCallback onSave;

  const RulesHeaderScope({
    super.key,
    required this.state,
    required this.selectedScope,
    required this.onScopeChanged,
    required this.onSave,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Breadcrumbs
        Row(
          children: [
            Text(
              'System / Automation Engines / ',
              style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
            ),
            Text(
              'Rules & Release Policies',
              style: ObsidianTypography.monoCode(color: ObsidianColors.primary)
                  .copyWith(fontWeight: FontWeight.w600),
            ),
          ],
        ),
        const SizedBox(height: 8),

        // Title and Toolbar
        Wrap(
          alignment: WrapAlignment.spaceBetween,
          crossAxisAlignment: WrapCrossAlignment.start,
          spacing: 16,
          runSpacing: 12,
          children: [
            Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      'Rules & Automation Policies',
                      style: ObsidianTypography.headlineLg(),
                    ),
                    const SizedBox(width: 10),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 2,
                      ),
                      decoration: BoxDecoration(
                        color: ObsidianColors.primary.withOpacity(0.12),
                        borderRadius: BorderRadius.circular(12),
                        border: Border.all(
                          color: ObsidianColors.primary.withOpacity(0.25),
                        ),
                      ),
                      child: Text(
                        'LIVE ENGINE',
                        style: ObsidianTypography.labelBadge(
                          color: ObsidianColors.primary,
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  'Configure automated parsing hierarchies, release scoring bonuses, codec filters, and ingest gating criteria.',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurfaceVariant,
                  ),
                ),
              ],
            ),

            // Scope Selector & Buttons
            Wrap(
              spacing: 8,
              runSpacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                // Scope Dropdown
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 4,
                  ),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(10),
                    border: Border.all(color: ObsidianColors.border),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        'Scope: ',
                        style: ObsidianTypography.monoCode(
                          color: ObsidianColors.outline,
                        ),
                      ),
                      DropdownButtonHideUnderline(
                        child: DropdownButton<String>(
                          value:
                              (selectedScope == 'global' ||
                                  state.shows.any(
                                    (s) => s.malId.toString() == selectedScope,
                                  ))
                              ? selectedScope
                              : 'global',
                          dropdownColor: ObsidianColors.surfaceContainerHigh,
                          icon: const Icon(
                            Icons.expand_more_rounded,
                            size: 16,
                            color: ObsidianColors.outline,
                          ),
                          style: ObsidianTypography.titleSm(
                            color: ObsidianColors.onSurface,
                          ),
                          items: [
                            const DropdownMenuItem(
                              value: 'global',
                              child: Text('Global Rules (Default)'),
                            ),
                            ...state.shows.map(
                              (s) => DropdownMenuItem(
                                value: s.malId.toString(),
                                child: Text('Override: ${s.title}'),
                              ),
                            ),
                          ],
                          onChanged: (val) {
                            if (val != null) onScopeChanged(val);
                          },
                        ),
                      ),
                      const SizedBox(width: 6),
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 6,
                          vertical: 1.5,
                        ),
                        decoration: BoxDecoration(
                          color: ObsidianColors.tertiary.withOpacity(0.18),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          selectedScope == 'global'
                              ? 'Active on all shows'
                              : 'Single show override',
                          style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.tertiary,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),

                // Test RSS Button
                OutlinedButton.icon(
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(
                        content: Text(
                          'Testing indexers: Nyaa.si reached successfully',
                        ),
                        backgroundColor: ObsidianColors.surfaceContainerHigh,
                      ),
                    );
                  },
                  icon: const Icon(
                    Icons.science_rounded,
                    size: 16,
                    color: ObsidianColors.tertiary,
                  ),
                  label: const Text('Test RSS Feed'),
                  style: OutlinedButton.styleFrom(
                    side: const BorderSide(color: ObsidianColors.border),
                    foregroundColor: ObsidianColors.onSurface,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                ),

                // Save Button
                ElevatedButton.icon(
                  onPressed: onSave,
                  icon: const Icon(
                    Icons.check_rounded,
                    size: 18,
                    color: ObsidianColors.onPrimaryContainer,
                  ),
                  label: const Text('Save Changes'),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: ObsidianColors.primaryContainer,
                    foregroundColor: ObsidianColors.onPrimaryContainer,
                    elevation: 0,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                ),
              ],
            ),
          ],
        ),
      ],
    );
  }
}
