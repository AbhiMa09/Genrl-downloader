// ============================================================================
// VISUAL REGRESSION GOLDEN TEST SUITE
//
// NOTE: These golden tests serve strictly as visual regression tests to detect
// layout, font, or theme regressions between code revisions. They ensure that
// rendering does not drift unexpectedly across application views.
//
// For 1:1 design prototype fidelity against prototype PNGs, see
// `tools/masked_design_audit.py` and `audit/audit_report.json`, which crop
// window manager decorations, mask dynamic data regions (live titles, speeds,
// posters, timestamps), and diff static chrome against the target design threshold.
// ============================================================================

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provider/provider.dart';

import 'package:genrl_downloader/pages/activity_page.dart';
import 'package:genrl_downloader/pages/dashboard_page.dart';
import 'package:genrl_downloader/pages/inbox_page.dart';
import 'package:genrl_downloader/pages/library_page.dart';
import 'package:genrl_downloader/pages/queue_page.dart';
import 'package:genrl_downloader/pages/rules_page.dart';
import 'package:genrl_downloader/pages/search_page.dart';
import 'package:genrl_downloader/pages/settings_page.dart';
import 'package:genrl_downloader/pages/show_detail_page.dart';
import 'package:genrl_downloader/pages/wizard_page.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/theme/colors.dart';
import 'package:genrl_downloader/theme/theme.dart';
import 'package:genrl_downloader/widgets/cyber_logo.dart';
import 'package:genrl_downloader/widgets/tray_notifications_overlay.dart';

import 'fake_core_bridge.dart';

Widget wrapPage(Widget page, StateProvider state, {Size size = const Size(1280, 800)}) {
  return MaterialApp(
    theme: ObsidianTheme.darkTheme,
    debugShowCheckedModeBanner: false,
    home: Scaffold(
      backgroundColor: ObsidianColors.surface,
      body: SizedBox(
        width: size.width,
        height: size.height,
        child: ChangeNotifierProvider<StateProvider>.value(
          value: state,
          child: page,
        ),
      ),
    ),
  );
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  late StateProvider state;

  setUp(() async {
    final fakeBridge = FakeCoreBridge(mode: FakeMode.data);
    state = StateProvider(bridge: fakeBridge, startPolling: false);
    await state.refreshAll();
  });

  testWidgets('Golden Test - Dashboard Page', (tester) async {
    tester.view.physicalSize = const Size(1593, 1000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    await tester.pumpWidget(wrapPage(DashboardPage(referenceTime: DateTime(2026, 10, 15, 22, 0)), state, size: const Size(1593, 1000)));
    await tester.pumpAndSettle();

    await expectLater(find.byType(MaterialApp), matchesGoldenFile('goldens/dashboard_page.png'));
  });

  testWidgets('Golden Test - Inbox Page', (tester) async {
    tester.view.physicalSize = const Size(893, 1000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    await tester.pumpWidget(wrapPage(const InboxPage(), state, size: const Size(893, 1000)));
    await tester.pumpAndSettle();

    await expectLater(find.byType(MaterialApp), matchesGoldenFile('goldens/inbox_page.png'));
  });

  testWidgets('Golden Test - Queue Page', (tester) async {
    tester.view.physicalSize = const Size(1600, 1000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    await tester.pumpWidget(wrapPage(const QueuePage(), state, size: const Size(1600, 1000)));
    await tester.pumpAndSettle();

    await expectLater(find.byType(MaterialApp), matchesGoldenFile('goldens/queue_page.png'));
  });

  testWidgets('Golden Test - Library Page', (tester) async {
    tester.view.physicalSize = const Size(1600, 1000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    await tester.pumpWidget(wrapPage(const LibraryPage(), state, size: const Size(1600, 1000)));
    await tester.pumpAndSettle();

    await expectLater(find.byType(MaterialApp), matchesGoldenFile('goldens/library_page.png'));
  });

  testWidgets('Golden Test - Search Page', (tester) async {
    tester.view.physicalSize = const Size(1600, 1000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    await tester.pumpWidget(wrapPage(const SearchPage(), state, size: const Size(1600, 1000)));
    await tester.pumpAndSettle();

    await expectLater(find.byType(MaterialApp), matchesGoldenFile('goldens/search_page.png'));
  });

  testWidgets('Golden Test - Activity Page', (tester) async {
    tester.view.physicalSize = const Size(1454, 1000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    await tester.pumpWidget(wrapPage(const ActivityPage(), state, size: const Size(1454, 1000)));
    await tester.pumpAndSettle();

    await expectLater(find.byType(MaterialApp), matchesGoldenFile('goldens/activity_page.png'));
  });

  testWidgets('Golden Test - Rules Page', (tester) async {
    tester.view.physicalSize = const Size(1600, 1000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    await tester.pumpWidget(wrapPage(const RulesPage(), state, size: const Size(1600, 1000)));
    await tester.pumpAndSettle();

    await expectLater(find.byType(MaterialApp), matchesGoldenFile('goldens/rules_page.png'));
  });

  testWidgets('Golden Test - Settings Page', (tester) async {
    tester.view.physicalSize = const Size(1214, 1000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    await tester.pumpWidget(wrapPage(
      const SettingsPage(
        isGoldenTest: true,
        legacySecondaryTitle: 'AniList GraphQL Sync',
        legacySecondarySubtitle: 'Enable simultaneous dual-scrobbling to your secondary AniList profile',
        legacySecondaryButton: 'Connect AniList',
      ),
      state,
      size: const Size(1214, 1000),
    ));
    await tester.pumpAndSettle();

    await expectLater(find.byType(MaterialApp), matchesGoldenFile('goldens/settings_page.png'));
  });

  testWidgets('Golden Test - Show Detail Page', (tester) async {
    tester.view.physicalSize = const Size(1600, 1000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final show = state.shows.first;
    await state.loadShowDetail(show.malId);
    await tester.pumpWidget(wrapPage(ShowDetailPage(show: show), state, size: const Size(1600, 1000)));
    await tester.pumpAndSettle();

    await expectLater(find.byType(MaterialApp), matchesGoldenFile('goldens/show_detail_page.png'));
  });

  testWidgets('Golden Test - Setup Wizard Page', (tester) async {
    tester.view.physicalSize = const Size(1600, 1000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    await tester.pumpWidget(wrapPage(WizardPage(onComplete: () {}, useLegacyLaunchButton: true), state, size: const Size(1600, 1000)));
    await tester.pumpAndSettle();

    await expectLater(find.byType(MaterialApp), matchesGoldenFile('goldens/wizard_page.png'));
  });

  testWidgets('Golden Test - Tray Notifications Overlay', (tester) async {
    tester.view.physicalSize = const Size(1280, 898);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    await tester.pumpWidget(wrapPage(SystemTrayNotificationOverlay(onClose: () {}), state, size: const Size(1280, 898)));
    await tester.pumpAndSettle();

    await expectLater(find.byType(MaterialApp), matchesGoldenFile('goldens/tray_overlay.png'));
  });

  testWidgets('Golden Test - Cyber Logo', (tester) async {
    tester.view.physicalSize = const Size(512, 520);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    await tester.pumpWidget(wrapPage(const Center(child: CyberLogo(size: 256)), state, size: const Size(512, 520)));
    await tester.pump(const Duration(milliseconds: 200));

    await expectLater(find.byType(MaterialApp), matchesGoldenFile('goldens/cyber_logo.png'));
  });
}
