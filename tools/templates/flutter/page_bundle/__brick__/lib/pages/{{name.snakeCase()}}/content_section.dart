import 'package:flutter/material.dart';
import '../../models/{{name.snakeCase()}}_model.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../theme/spacing.dart';

/// Content list or grid section for {{name.pascalCase()}}.
class {{name.pascalCase()}}ContentSection extends StatelessWidget {
  final List<{{name.pascalCase()}}Item> items;

  const {{name.pascalCase()}}ContentSection({
    super.key,
    required this.items,
  });

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      return Container(
        padding: const EdgeInsets.all(ObsidianSpacing.spaceXl),
        alignment: Alignment.center,
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainerLowest,
          borderRadius: BorderRadius.circular(ObsidianRadius.defaultRadius),
        ),
        child: Text(
          'No {{name.titleCase()}} records available',
          style: ObsidianTypography.bodyMd().copyWith(color: ObsidianColors.outline),
        ),
      );
    }

    return ListView.separated(
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      itemCount: items.length,
      separatorBuilder: (_, __) => const SizedBox(height: ObsidianSpacing.spaceSm),
      itemBuilder: (context, index) {
        final item = items[index];
        return Container(
          padding: const EdgeInsets.all(ObsidianSpacing.spaceSm),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(ObsidianRadius.defaultRadius),
            border: Border.all(color: ObsidianColors.outlineVariant),
          ),
          child: ListTile(
            title: Text(item.title, style: ObsidianTypography.titleSm()),
            subtitle: Text(item.description, style: ObsidianTypography.bodySm()),
          ),
        );
      },
    );
  }
}
