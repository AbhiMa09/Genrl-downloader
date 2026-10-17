import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/pages/dashboard/dashboard_ingestion_strip.dart';
import 'package:genrl_downloader/pages/dashboard_page.dart';
import 'package:genrl_downloader/pages/wizard/step_mal_account.dart';
import 'package:genrl_downloader/pages/wizard/step_qbittorrent.dart';
import 'package:genrl_downloader/pages/wizard/step_quality_groups.dart';
import 'package:genrl_downloader/pages/wizard/step_root_path.dart';
import 'package:genrl_downloader/pages/wizard/wizard_daily_cap_warning.dart';
import 'package:genrl_downloader/pages/wizard/wizard_headline_card.dart';
import 'package:genrl_downloader/pages/wizard/wizard_page.dart';
import 'package:genrl_downloader/pages/wizard/wizard_series_manifest.dart';
import 'package:genrl_downloader/services/core_bridge.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/theme/colors.dart';
import 'package:genrl_downloader/theme/theme.dart';
import 'package:genrl_downloader/widgets/loading/ingestion_progress.dart';
import 'package:google_fonts/google_fonts.dart';
import 'package:provider/provider.dart';

const String kScratchDbDir =
    '/home/genrl/.gemini/antigravity/brain/d325a24e-cc96-4f27-959a-64ff796d34bc/scratch/test_db';
const String kScreenshotsDir =
    '/home/genrl/.gemini/antigravity/brain/d325a24e-cc96-4f27-959a-64ff796d34bc/scratch/screenshots';

Future<void> _capturePng(
  WidgetTester tester,
  Finder finder,
  String filename,
) async {
  await tester.runAsync(() async {
    final element = finder.evaluate().first;
    final RenderRepaintBoundary boundary =
        element.renderObject! as RenderRepaintBoundary;
    final ui.Image image = await boundary.toImage(pixelRatio: 1.0);
    final byteData = await image.toByteData(format: ui.ImageByteFormat.png);
    final pngBytes = byteData!.buffer.asUint8List();
    final file = File('$kScreenshotsDir/$filename');
    file.writeAsBytesSync(pngBytes);
  });
}

Widget _wrap(
  Widget child,
  StateProvider state, {
  Size size = const Size(1600, 1000),
}) {
  return MaterialApp(
    theme: ObsidianTheme.darkTheme,
    debugShowCheckedModeBanner: false,
    home: Scaffold(
      backgroundColor: ObsidianColors.surface,
      body: Center(
        child: SizedBox(
          width: size.width,
          height: size.height,
          child: ChangeNotifierProvider<StateProvider>.value(
            value: state,
            child: RepaintBoundary(
              key: const ValueKey('capture_root'),
              child: child,
            ),
          ),
        ),
      ),
    ),
  );
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  GoogleFonts.config.allowRuntimeFetching = false;

  late CoreBridge bridge;
  late StateProvider state;

  setUpAll(() {
    Directory(kScreenshotsDir).createSync(recursive: true);
  });

  setUp(() async {
    bridge = CoreBridge(customDataDir: kScratchDbDir);
    state = StateProvider(bridge: bridge, startPolling: false);
    await state.refreshAll();
  });

  testWidgets(
    'Walk 5 wizard steps with real scratch data, verify planned counts, and capture screenshots',
    (tester) async {
      tester.view.physicalSize = const Size(1600, 1000);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() => tester.view.resetPhysicalSize());

      bool completed = false;

      // 1. Walk 5 steps
      await tester.pumpWidget(
        _wrap(
          WizardPage(initialStep: 0, onComplete: () => completed = true),
          state,
        ),
      );
      await tester.pumpAndSettle();
      expect(completed, isFalse);

      // Step 0: MAL Account
      expect(find.byType(StepMalAccount), findsOneWidget);
      await tester.tap(find.text('Next Step'));
      await tester.pumpAndSettle();

      // Step 1: qBittorrent
      expect(find.byType(StepQbittorrent), findsOneWidget);
      await tester.tap(find.text('Next Step'));
      await tester.pumpAndSettle();

      // Step 2: Root Path
      expect(find.byType(StepRootPath), findsOneWidget);
      await tester.tap(find.text('Next Step'));
      await tester.pumpAndSettle();

      // Step 3: Quality & Groups
      expect(find.byType(StepQualityGroups), findsOneWidget);
      await tester.tap(find.text('Next Step'));
      await tester.pumpAndSettle();

      // Step 4: Review & Launch
      expect(find.byType(WizardHeadlineCard), findsOneWidget);
      expect(find.byType(WizardDailyCapWarning), findsOneWidget);
      expect(find.byType(WizardSeriesManifest), findsOneWidget);

      // Verify real planned counts
      expect(state.plannedEpisodesCount, 11);
      expect(state.plannedShowsCount, 11);
      expect(state.estimatedStorageGb, 13.2);
      expect(state.dailyTorrentCap, 30);

      // Verify bypass daily cap is off by default
      final checkbox = tester.widget<Checkbox>(find.byType(Checkbox));
      expect(checkbox.value, isFalse);

      final rootFinder = find.byKey(const ValueKey('capture_root'));
      await _capturePng(tester, rootFinder, 'step_4_review_and_launch.png');

      // 2. Stage 1: MAL Sync
      state.setIngestionProgressForTesting(
        stage: IngestionStage.malSync,
        isIngesting: true,
        processedCount: 2,
        totalCount: 11,
        etaSeconds: 40,
      );
      await tester.pump();
      await _capturePng(tester, rootFinder, 'stage_1_mal_sync.png');

      // Stage 2: Metadata
      state.setIngestionProgressForTesting(
        stage: IngestionStage.metadata,
        isIngesting: true,
        processedCount: 5,
        totalCount: 11,
        etaSeconds: 28,
      );
      await tester.pump();
      await _capturePng(tester, rootFinder, 'stage_2_metadata.png');

      // Stage 3: Plan Queue
      state.setIngestionProgressForTesting(
        stage: IngestionStage.planQueue,
        isIngesting: true,
        processedCount: 9,
        totalCount: 11,
        etaSeconds: 14,
      );
      await tester.pump();
      await _capturePng(tester, rootFinder, 'stage_3_plan_queue.png');

      // Stage 4: Create Jobs
      state.setIngestionProgressForTesting(
        stage: IngestionStage.createJobs,
        isIngesting: true,
        processedCount: 10,
        totalCount: 11,
        etaSeconds: 5,
      );
      await tester.pump();
      await _capturePng(tester, rootFinder, 'stage_4_create_jobs.png');

      // Stage Failure + Real Error + Retry
      state.setIngestionProgressForTesting(
        stage: IngestionStage.failed,
        isIngesting: false,
        processedCount: 5,
        totalCount: 11,
        errorMessage: 'Nyaa gateway timeout (HTTP 504 Gateway Timeout)',
      );
      await tester.pump();
      expect(find.text('Retry Stage'), findsOneWidget);
      await _capturePng(tester, rootFinder, 'stage_failed_retry.png');

      // Run in background (Ctrl+B) -> Dashboard continues
      state.setIngestionProgressForTesting(
        stage: IngestionStage.createJobs,
        isIngesting: true,
        processedCount: 10,
        totalCount: 11,
        etaSeconds: 4,
      );
      state.backgroundIngestion();
      expect(state.isIngestionBackgrounded, isTrue);

      await tester.pumpWidget(
        _wrap(
          DashboardPage(referenceTime: DateTime(2026, 10, 15, 22, 0)),
          state,
        ),
      );
      await tester.pump();
      expect(find.byType(DashboardIngestionStrip), findsOneWidget);
      await _capturePng(tester, rootFinder, 'dashboard_background_strip.png');

      // Ingestion completion -> Dashboard
      state.setIngestionProgressForTesting(
        stage: IngestionStage.completed,
        isIngesting: false,
        processedCount: 11,
        totalCount: 11,
        etaSeconds: 0,
      );
      state.setFirstRunCompleted(true);
      await tester.pumpWidget(
        _wrap(
          DashboardPage(referenceTime: DateTime(2026, 10, 15, 22, 0)),
          state,
        ),
      );
      await tester.pump();
      expect(find.byType(DashboardPage), findsOneWidget);
      await _capturePng(tester, rootFinder, 'dashboard_completed.png');
    },
  );

  test('Run setup again keeps data without deleting or duplicating', () async {
    // Check initial row counts in scratch DB
    final showsBefore = await bridge.getShows(status: 'all');
    final queueBefore = await bridge.getQueue();
    expect(showsBefore.length, 1040);
    expect(queueBefore.length, 7);

    // Call resetFirstRun
    await state.resetFirstRun();
    expect(state.firstRunCompleted, isFalse);

    // Verify row counts after reset
    final showsAfter = await bridge.getShows(status: 'all');
    final queueAfter = await bridge.getQueue();
    expect(showsAfter.length, 1040);
    expect(queueAfter.length, 7);

    // Ensure no duplicates or deletions
    final idsBefore = showsBefore.map((s) => s.malId).toSet();
    final idsAfter = showsAfter.map((s) => s.malId).toSet();
    expect(idsAfter, equals(idsBefore));

    final jobIdsBefore = queueBefore.map((q) => q.id).toSet();
    final jobIdsAfter = queueAfter.map((q) => q.id).toSet();
    expect(jobIdsAfter, equals(jobIdsBefore));
  });
}
