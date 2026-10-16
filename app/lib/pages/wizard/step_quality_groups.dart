import 'package:flutter/material.dart';

import '../../theme/colors.dart';
import '../../theme/typography.dart';

class StepQualityGroups extends StatefulWidget {
  const StepQualityGroups({super.key});

  @override
  State<StepQualityGroups> createState() => _StepQualityGroupsState();
}

class _StepQualityGroupsState extends State<StepQualityGroups> {
  String _selectedRes = '1080p';
  final Set<String> _selectedGroups = {
    'SubsPlease',
    'Erai-raws',
    'ASW',
    'Judas',
    'Golumpa',
  };

  final List<String> _availableGroups = [
    'SubsPlease',
    'Erai-raws',
    'ASW',
    'Judas',
    'Golumpa',
    'HorribleSubs',
    'Commie',
  ];

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Quality & Release Group Priority',
          style: ObsidianTypography.headlineMd(),
        ),
        const SizedBox(height: 6),
        Text(
          'Set your default video resolution and preferred fansub release groups.',
          style: ObsidianTypography.bodySm(
            color: ObsidianColors.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 20),
        Text('Target Resolution', style: ObsidianTypography.titleSm()),
        const SizedBox(height: 8),
        Row(
          children: ['1080p', '720p', '2160p (4K)'].map((res) {
            final isSelected = _selectedRes == res;
            return Padding(
              padding: const EdgeInsets.only(right: 8),
              child: ChoiceChip(
                label: Text(res),
                selected: isSelected,
                selectedColor: ObsidianColors.primaryContainer,
                onSelected: (val) {
                  if (val) setState(() => _selectedRes = res);
                },
              ),
            );
          }).toList(),
        ),
        const SizedBox(height: 20),
        Text(
          'Preferred Groups (Prioritized Match)',
          style: ObsidianTypography.titleSm(),
        ),
        const SizedBox(height: 8),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: _availableGroups.map((group) {
            final isSelected = _selectedGroups.contains(group);
            return FilterChip(
              label: Text(group),
              selected: isSelected,
              selectedColor: ObsidianColors.tertiaryContainer,
              onSelected: (val) {
                setState(() {
                  if (val) {
                    _selectedGroups.add(group);
                  } else {
                    _selectedGroups.remove(group);
                  }
                });
              },
            );
          }).toList(),
        ),
      ],
    );
  }
}
