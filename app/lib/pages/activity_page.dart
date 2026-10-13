import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/activity_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';

class ActivityPage extends StatefulWidget {
  const ActivityPage({super.key});

  @override
  State<ActivityPage> createState() => _ActivityPageState();
}

class _ActivityPageState extends State<ActivityPage> {
  bool _errorsOnly = false;

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    final filteredEvents = state.events.where((e) {
      if (_errorsOnly) {
        return e.level == ActivityLevel.warn || e.level == ActivityLevel.error;
      }
      return true;
    }).toList();

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Text('Activity & Telemetry Log', style: ObsidianTypography.headlineLg()),
              const Spacer(),
              _FilterButton(
                label: 'All Events',
                isSelected: !_errorsOnly,
                onTap: () => setState(() => _errorsOnly = false),
              ),
              const SizedBox(width: 8),
              _FilterButton(
                label: 'Warnings & Errors Only',
                isSelected: _errorsOnly,
                onTap: () => setState(() => _errorsOnly = true),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            'Auditable timeline of system state changes, candidate searches, downloads, and finalizations.',
            style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
          ),
          const SizedBox(height: 20),

          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLow,
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: ObsidianColors.border),
            ),
            child: ListView.separated(
              shrinkWrap: true,
              physics: const NeverScrollableScrollPhysics(),
              itemCount: filteredEvents.length,
              separatorBuilder: (_, __) =>
                  const Divider(height: 16, color: ObsidianColors.border),
              itemBuilder: (context, index) {
                final event = filteredEvents[index];

                Color levelColor = ObsidianColors.primary;
                String levelText = 'INFO';
                if (event.level == ActivityLevel.warn) {
                  levelColor = ObsidianColors.warning;
                  levelText = 'WARN';
                } else if (event.level == ActivityLevel.error) {
                  levelColor = ObsidianColors.error;
                  levelText = 'ERROR';
                }

                return Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      event.formattedTime,
                      style: ObsidianTypography.monoCode(
                          color: ObsidianColors.outline),
                    ),
                    const SizedBox(width: 12),
                    Container(
                      padding: const EdgeInsets.symmetric(
                          horizontal: 5, vertical: 1.5),
                      decoration: BoxDecoration(
                        color: levelColor.withOpacity(0.18),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        levelText,
                        style: ObsidianTypography.labelBadge(color: levelColor),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        event.message,
                        style: ObsidianTypography.bodySm(
                            color: ObsidianColors.onSurface),
                      ),
                    ),
                  ],
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}

class _FilterButton extends StatelessWidget {
  final String label;
  final bool isSelected;
  final VoidCallback onTap;

  const _FilterButton({
    required this.label,
    required this.isSelected,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        decoration: BoxDecoration(
          color: isSelected
              ? ObsidianColors.surfaceContainerHigh
              : ObsidianColors.surfaceContainerLowest,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isSelected
                ? ObsidianColors.primary.withOpacity(0.3)
                : ObsidianColors.border,
          ),
        ),
        child: Text(
          label,
          style: isSelected
              ? ObsidianTypography.bodySm(color: ObsidianColors.primary)
                  .copyWith(fontWeight: FontWeight.w600)
              : ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
        ),
      ),
    );
  }
}
