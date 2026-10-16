import 'package:flutter/material.dart';

import 'colors.dart';
import 'typography.dart';

/// Obsidian Violet Desktop Theme Configuration
class ObsidianTheme {
  static ThemeData get darkTheme {
    return ThemeData(
      brightness: Brightness.dark,
      scaffoldBackgroundColor: ObsidianColors.surface,
      primaryColor: ObsidianColors.primary,
      canvasColor: ObsidianColors.surfaceContainerLow,
      cardColor: ObsidianColors.surfaceContainerLow,
      dividerColor: ObsidianColors.border,
      colorScheme: const ColorScheme.dark(
        surface: ObsidianColors.surface,
        onSurface: ObsidianColors.onSurface,
        primary: ObsidianColors.primary,
        onPrimary: ObsidianColors.onPrimary,
        primaryContainer: ObsidianColors.primaryContainer,
        onPrimaryContainer: ObsidianColors.onPrimaryContainer,
        secondary: ObsidianColors.secondary,
        onSecondary: ObsidianColors.onSecondary,
        tertiary: ObsidianColors.tertiary,
        onTertiary: ObsidianColors.onTertiary,
        error: ObsidianColors.error,
        onError: ObsidianColors.onError,
        outline: ObsidianColors.outline,
        outlineVariant: ObsidianColors.outlineVariant,
      ),
      cardTheme: CardThemeData(
        color: ObsidianColors.surfaceContainerLow,
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: const BorderSide(color: ObsidianColors.border, width: 1),
        ),
      ),
      scrollbarTheme: ScrollbarThemeData(
        thumbColor: WidgetStateProperty.all(
          ObsidianColors.surfaceContainerHighest,
        ),
        radius: const Radius.circular(4),
        thickness: WidgetStateProperty.all(6),
      ),
      textTheme: TextTheme(
        headlineLarge: ObsidianTypography.headlineLg(),
        headlineMedium: ObsidianTypography.headlineMd(),
        titleSmall: ObsidianTypography.titleSm(),
        bodyMedium: ObsidianTypography.bodyMd(),
        bodySmall: ObsidianTypography.bodySm(),
      ),
    );
  }
}
