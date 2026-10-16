import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/theme/colors.dart';
import 'package:genrl_downloader/theme/spacing.dart';
import 'package:genrl_downloader/theme/typography.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('DESIGN.md Token Compliance Verification', () {
    test('All Colors in ObsidianColors match DESIGN.md exactly', () {
      expect(ObsidianColors.surface.value, equals(0xFF10141A));
      expect(ObsidianColors.surfaceDim.value, equals(0xFF10141A));
      expect(ObsidianColors.surfaceBright.value, equals(0xFF353940));
      expect(ObsidianColors.surfaceContainerLowest.value, equals(0xFF0A0E14));
      expect(ObsidianColors.surfaceContainerLow.value, equals(0xFF181C22));
      expect(ObsidianColors.surfaceContainer.value, equals(0xFF1C2026));
      expect(ObsidianColors.surfaceContainerHigh.value, equals(0xFF262A31));
      expect(ObsidianColors.surfaceContainerHighest.value, equals(0xFF31353C));
      expect(ObsidianColors.onSurface.value, equals(0xFFDFE2EB));
      expect(ObsidianColors.onSurfaceVariant.value, equals(0xFFCBC3D7));
      expect(ObsidianColors.outline.value, equals(0xFF958EA0));
      expect(ObsidianColors.outlineVariant.value, equals(0xFF494454));
      expect(ObsidianColors.primary.value, equals(0xFFD0BCFF));
      expect(ObsidianColors.onPrimary.value, equals(0xFF3C0091));
      expect(ObsidianColors.primaryContainer.value, equals(0xFFA078FF));
      expect(ObsidianColors.onPrimaryContainer.value, equals(0xFF340080));
      expect(ObsidianColors.secondary.value, equals(0xFFD2BBFF));
      expect(ObsidianColors.onSecondary.value, equals(0xFF3F008E));
      expect(ObsidianColors.secondaryContainer.value, equals(0xFF6001D1));
      expect(ObsidianColors.onSecondaryContainer.value, equals(0xFFC9AEFF));
      expect(ObsidianColors.tertiary.value, equals(0xFF7BD0FF));
      expect(ObsidianColors.onTertiary.value, equals(0xFF00354A));
      expect(ObsidianColors.tertiaryContainer.value, equals(0xFF009BD1));
      expect(ObsidianColors.error.value, equals(0xFFFFB4AB));
      expect(ObsidianColors.onError.value, equals(0xFF690005));
      expect(ObsidianColors.errorContainer.value, equals(0xFF93000A));
      expect(ObsidianColors.onErrorContainer.value, equals(0xFFFFDAD6));
      expect(ObsidianColors.inverseSurface.value, equals(0xFFDFE2EB));
      expect(ObsidianColors.inverseOnSurface.value, equals(0xFF2D3137));
      expect(ObsidianColors.inversePrimary.value, equals(0xFF6D3BD7));
      expect(ObsidianColors.primaryFixed.value, equals(0xFFE9DDFF));
      expect(ObsidianColors.onPrimaryFixed.value, equals(0xFF23005C));
      expect(ObsidianColors.onPrimaryFixedVariant.value, equals(0xFF5516BE));
      expect(ObsidianColors.secondaryFixed.value, equals(0xFFEADDFF));
      expect(ObsidianColors.secondaryFixedDim.value, equals(0xFFD2BBFF));
      expect(ObsidianColors.onSecondaryFixed.value, equals(0xFF25005A));
      expect(ObsidianColors.onSecondaryFixedVariant.value, equals(0xFF5A00C6));
      expect(ObsidianColors.tertiaryFixed.value, equals(0xFFC4E7FF));
      expect(ObsidianColors.tertiaryFixedDim.value, equals(0xFF7BD0FF));
      expect(ObsidianColors.onTertiaryFixed.value, equals(0xFF001E2C));
      expect(ObsidianColors.onTertiaryFixedVariant.value, equals(0xFF004C69));
      expect(ObsidianColors.onTertiaryContainer.value, equals(0xFF002D40));
      expect(ObsidianColors.background.value, equals(0xFF10141A));
      expect(ObsidianColors.onBackground.value, equals(0xFFDFE2EB));
      expect(ObsidianColors.surfaceVariant.value, equals(0xFF31353C));
      expect(ObsidianColors.border.value, equals(0xFF30363D));
    });

    test('Spacing and Radius tokens match DESIGN.md scale exactly', () {
      expect(ObsidianSpacing.space2xs, equals(2.0));
      expect(ObsidianSpacing.spaceXs, equals(4.0));
      expect(ObsidianSpacing.spaceSm, equals(8.0));
      expect(ObsidianSpacing.gutter, equals(12.0));
      expect(ObsidianSpacing.spaceMd, equals(12.0));
      expect(ObsidianSpacing.margin, equals(16.0));
      expect(ObsidianSpacing.spaceLg, equals(16.0));
      expect(ObsidianSpacing.spaceXl, equals(24.0));
      expect(ObsidianSpacing.space2xl, equals(32.0));

      expect(ObsidianRadius.sm, equals(4.0));
      expect(ObsidianRadius.defaultRadius, equals(8.0));
      expect(ObsidianRadius.md, equals(12.0));
      expect(ObsidianRadius.lg, equals(16.0));
      expect(ObsidianRadius.xl, equals(24.0));
      expect(ObsidianRadius.full, equals(9999.0));
    });

    test(
      'Typography tokens match DESIGN.md font family, size and letter spacing',
      () {
        final hXl = ObsidianTypography.headlineXl();
        expect(hXl.fontSize, equals(32.0));
        expect(hXl.fontWeight, equals(FontWeight.w700));
        expect(hXl.letterSpacing, closeTo(-0.64, 0.01));

        final hLg = ObsidianTypography.headlineLg();
        expect(hLg.fontSize, equals(24.0));
        expect(hLg.fontWeight, equals(FontWeight.w600));
        expect(hLg.letterSpacing, closeTo(-0.36, 0.01));

        final hMd = ObsidianTypography.headlineMd();
        expect(hMd.fontSize, equals(20.0));
        expect(hMd.fontWeight, equals(FontWeight.w600));
        expect(hMd.letterSpacing, closeTo(-0.2, 0.01));

        final titleSm = ObsidianTypography.titleSm();
        expect(titleSm.fontSize, equals(15.0));
        expect(titleSm.fontWeight, equals(FontWeight.w600));
        expect(titleSm.letterSpacing, equals(0.0));

        final bodyMd = ObsidianTypography.bodyMd();
        expect(bodyMd.fontSize, equals(14.0));
        expect(bodyMd.fontWeight, equals(FontWeight.w400));
        expect(bodyMd.letterSpacing, equals(0.0));

        final bodySm = ObsidianTypography.bodySm();
        expect(bodySm.fontSize, equals(13.0));
        expect(bodySm.fontWeight, equals(FontWeight.w400));
        expect(bodySm.letterSpacing, equals(0.0));

        final monoMetric = ObsidianTypography.monoMetric();
        expect(monoMetric.fontSize, equals(13.0));
        expect(monoMetric.fontWeight, equals(FontWeight.w500));
        expect(monoMetric.letterSpacing, closeTo(-0.26, 0.01));

        final monoCode = ObsidianTypography.monoCode();
        expect(monoCode.fontSize, equals(12.0));
        expect(monoCode.fontWeight, equals(FontWeight.w400));
        expect(monoCode.letterSpacing, equals(0.0));

        final labelBadge = ObsidianTypography.labelBadge();
        expect(labelBadge.fontSize, equals(11.0));
        expect(labelBadge.fontWeight, equals(FontWeight.w600));
        expect(labelBadge.letterSpacing, closeTo(0.44, 0.01));
      },
    );
  });
}
