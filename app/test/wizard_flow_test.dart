import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provider/provider.dart';
import 'package:genrl_downloader/main.dart';
import 'package:genrl_downloader/pages/wizard_page.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/theme/theme.dart';
import 'package:genrl_downloader/widgets/loading/ingestion_progress.dart';

import 'fake_core_bridge.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  late FakeCoreBridge fakeBridge;
  late StateProvider state;

  setUp(() {
    fakeBridge = FakeCoreBridge(mode: FakeMode.data);
    state = StateProvider(bridge: fakeBridge, startPolling: false);
  });

  Widget createTestApp({required Widget home}) {
    return ChangeNotifierProvider<StateProvider>.value(
      value: state,
      child: MaterialApp(theme: ObsidianTheme.darkTheme, home: home),
    );
  }

  testWidgets(
    'Wizard flow: launch -> step navigation -> launch ingestion -> done -> complete callback',
    (tester) async {
      tester.view.physicalSize = const Size(1600, 1000);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() => tester.view.resetPhysicalSize());

      bool isComplete = false;

      await tester.pumpWidget(
        createTestApp(
          home: WizardPage(initialStep: 4, onComplete: () => isComplete = true),
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.byType(WizardPage), findsOneWidget);
      final turnOnFinder = find.text('Turn automation on');
      expect(turnOnFinder, findsOneWidget);
      expect(find.text('Start with automation off'), findsOneWidget);

      await tester.ensureVisible(turnOnFinder);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      // Launch with automation ON
      await tester.tap(turnOnFinder);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      // Now IngestionProgress should be displayed inside the wizard
      expect(find.byType(IngestionProgress), findsOneWidget);

      // Pump frames to let async stages finish
      for (int i = 0; i < 20; i++) {
        await tester.pump(const Duration(milliseconds: 100));
      }

      // On completion, onComplete should be called
      expect(isComplete, isTrue);
    },
  );

  testWidgets(
    'Wizard backgrounding: clicking Run in background completes wizard immediately',
    (tester) async {
      tester.view.physicalSize = const Size(1600, 1000);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() => tester.view.resetPhysicalSize());

      bool isComplete = false;

      await tester.pumpWidget(
        createTestApp(
          home: WizardPage(initialStep: 4, onComplete: () => isComplete = true),
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      final startOffFinder = find.text('Start with automation off');
      expect(startOffFinder, findsOneWidget);
      await tester.ensureVisible(startOffFinder);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      // Launch
      await tester.tap(startOffFinder);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.byType(IngestionProgress), findsOneWidget);

      // Tap "Run in background"
      final bgFinder = find.text('Run in background (Ctrl+B)');
      expect(bgFinder, findsOneWidget);
      await tester.ensureVisible(bgFinder);
      await tester.pump();
      await tester.tap(bgFinder);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      expect(isComplete, isTrue);
    },
  );

  testWidgets('Ingestion stage failure displays error and Retry Stage button', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    bool retried = false;

    await tester.pumpWidget(
      createTestApp(
        home: Scaffold(
          body: IngestionProgress(
            stage: IngestionStage.failed,
            errorMessage: 'Tracker connection timed out',
            onRetry: () => retried = true,
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.text('Ingestion Stage Interrupted'), findsOneWidget);
    expect(find.text('Tracker connection timed out'), findsOneWidget);
    expect(find.text('Retry Stage'), findsOneWidget);

    await tester.tap(find.text('Retry Stage'));
    expect(retried, isTrue);
  });

  testWidgets('first_run_completed gating controls WizardPage vs MainShell', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    // When firstRunCompleted is false, app displays WizardPage
    state.setFirstRunCompleted(false);

    await tester.pumpWidget(GenrlApp(stateProvider: state));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.byType(WizardPage), findsOneWidget);
    expect(find.byType(MainShell), findsNothing);

    // When firstRunCompleted becomes true, app displays MainShell
    state.setFirstRunCompleted(true);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.byType(MainShell), findsOneWidget);
    expect(find.byType(WizardPage), findsNothing);

    // When resetFirstRun() is called, app returns to WizardPage
    await state.resetFirstRun();
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.byType(WizardPage), findsOneWidget);
    expect(find.byType(MainShell), findsNothing);
  });
}
