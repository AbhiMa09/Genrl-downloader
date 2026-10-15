import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../theme/colors.dart';
import '../../theme/spacing.dart';
import '../../services/cubit/{{name.snakeCase()}}_cubit.dart';
import '../../services/cubit/{{name.snakeCase()}}_state.dart';
import 'hero_section.dart';
import 'content_section.dart';

/// {{name.pascalCase()}} Page shell.
/// Follows modular structure: sub-sections split into dedicated widgets (<300 lines).
class {{name.pascalCase()}}Page extends StatelessWidget {
  const {{name.pascalCase()}}Page({super.key});

  @override
  Widget build(BuildContext context) {
    return ChangeNotifierProvider(
      create: (_) => {{name.pascalCase()}}Cubit()..load(),
      child: Scaffold(
        backgroundColor: ObsidianColors.surface,
        body: Consumer<{{name.pascalCase()}}Cubit>(
          builder: (context, cubit, _) {
            final state = cubit.state;
            if (state.isLoading) {
              return const Center(child: CircularProgressIndicator());
            }
            if (state.error != null) {
              return Center(
                child: Text(
                  'Error: ${state.error}',
                  style: TextStyle(color: ObsidianColors.error),
                ),
              );
            }

            return SingleChildScrollView(
              padding: const EdgeInsets.all(ObsidianSpacing.spaceLg),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  {{name.pascalCase()}}HeroSection(title: '{{name.pascalCase()}}'),
                  const SizedBox(height: ObsidianSpacing.spaceLg),
                  {{name.pascalCase()}}ContentSection(items: state.items),
                ],
              ),
            );
          },
        ),
      ),
    );
  }
}
