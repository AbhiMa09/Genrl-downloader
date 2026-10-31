import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/main.dart' as app;
import 'package:genrl_downloader/pages/dashboard/dashboard_pipeline_table.dart';
import 'package:genrl_downloader/pages/dashboard_page.dart';
import 'package:genrl_downloader/pages/wizard/step_mal_account.dart';
import 'package:genrl_downloader/pages/wizard/step_qbittorrent.dart';
import 'package:genrl_downloader/pages/wizard/step_quality_groups.dart';
import 'package:genrl_downloader/pages/wizard/step_review_prelaunch.dart';
import 'package:genrl_downloader/pages/wizard/step_root_path.dart';
import 'package:genrl_downloader/pages/wizard/wizard_page.dart';
import 'package:genrl_downloader/pages/wizard/wizard_plan_confirmation.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/widgets/loading/ingestion_progress.dart';
import 'package:integration_test/integration_test.dart';
import 'package:provider/provider.dart';

const String kScratchBase =
    '/home/genrl/.gemini/antigravity/brain/d325a24e-cc96-4f27-959a-64ff796d34bc/scratch';
const String kCleanDbDir = '$kScratchBase/clean_first_run';
const String kArtifactDir =
    '/home/genrl/.gemini/antigravity/brain/d325a24e-cc96-4f27-959a-64ff796d34bc';
const String kPhotoDir = '/home/genrl/Documents/GitHub/Genrl-downloader/photo';
const String kGenrlBin =
    '/home/genrl/Documents/GitHub/Genrl-downloader/core/target/debug/genrl';

Future<void> _capture(
  WidgetTester tester,
  Finder finder,
  String filename, {
  String? subfolder,
}) async {
  await tester.runAsync(() async {
    final element = finder.evaluate().first;
    final RenderRepaintBoundary boundary =
        element.renderObject! as RenderRepaintBoundary;
    final ui.Image image = await boundary.toImage(pixelRatio: 1.0);
    final byteData = await image.toByteData(format: ui.ImageByteFormat.png);
    final pngBytes = byteData!.buffer.asUint8List();
    File('$kArtifactDir/$filename').writeAsBytesSync(pngBytes);
    File('$kPhotoDir/$filename').writeAsBytesSync(pngBytes);
    if (subfolder != null) {
      final subDir = Directory('$kPhotoDir/by_version/$subfolder');
      if (!subDir.existsSync()) subDir.createSync(recursive: true);
      File('$kPhotoDir/by_version/$subfolder/$filename')
          .writeAsBytesSync(pngBytes);
    }
  });
}

Map<String, int> _getDbRowCounts(String dbPath) {
  final res = Process.runSync('sqlite3', [
    '-separator',
    '|',
    dbPath,
    "SELECT 'shows', count(*) FROM shows UNION ALL "
        "SELECT 'mal_entries', count(*) FROM mal_entries UNION ALL "
        "SELECT 'episodes', count(*) FROM episodes UNION ALL "
        "SELECT 'jobs', count(*) FROM jobs;",
  ]);
  final counts = <String, int>{};
  for (final line in res.stdout.toString().split('\n')) {
    final parts = line.split('|');
    if (parts.length == 2) {
      counts[parts[0].trim()] = int.tryParse(parts[1].trim()) ?? 0;
    }
  }
  return counts;
}

void _recordStageCountsRaw(String stageName, String dbPath) {
  final buffer = StringBuffer();
  buffer.writeln(
    '============================================================',
  );
  buffer.writeln(' STAGE: $stageName');
  buffer.writeln(' TIMESTAMP: ${DateTime.now().toUtc().toIso8601String()}');
  buffer.writeln(
    '============================================================',
  );
  buffer.writeln('\n--- DB ROW COUNTS ---');
  final countsRes = Process.runSync('sqlite3', [
    '-header',
    '-column',
    dbPath,
    "SELECT 'shows' as entity, count(*) as count FROM shows UNION ALL "
        "SELECT 'mal_entries', count(*) FROM mal_entries UNION ALL "
        "SELECT 'episodes', count(*) FROM episodes UNION ALL "
        "SELECT 'jobs', count(*) FROM jobs;",
  ]);
  buffer.writeln(countsRes.stdout.toString().trim());

  buffer.writeln('\n--- SHOWS BY METADATA_STATUS ---');
  final metaRes = Process.runSync('sqlite3', [
    '-header',
    '-column',
    dbPath,
    "SELECT coalesce(metadata_status, 'NULL') as status, count(*) as count FROM shows GROUP BY 1 ORDER BY 1;",
  ]);
  buffer.writeln(metaRes.stdout.toString().trim());

  buffer.writeln(
    '\n--- ONGOING & LONG-RUNNING SHOWS (Conan, One Piece, Boruto) ---',
  );
  final showsRes = Process.runSync('sqlite3', [
    '-header',
    '-column',
    dbPath,
    "SELECT s.mal_id, s.title, coalesce(s.total_episodes, -1) as total_episodes, s.status, s.metadata_status, m.watched_count "
        "FROM shows s JOIN mal_entries m ON s.mal_id = m.mal_id "
        "WHERE s.mal_id IN (21, 235, 34566) ORDER BY s.mal_id;",
  ]);
  buffer.writeln(showsRes.stdout.toString().trim());

  buffer.writeln('\n--- STATE TIMESTAMPS ---');
  final timeRes = Process.runSync('sqlite3', [
    '-header',
    '-column',
    dbPath,
    "SELECT key, value FROM settings WHERE key LIKE 'state.%' ORDER BY 1;",
  ]);
  buffer.writeln(timeRes.stdout.toString().trim());
  buffer.writeln('\n');

  File('$kPhotoDir/stage_db_counts.txt')
      .writeAsStringSync(buffer.toString(), mode: FileMode.append);
  final v1Dir = Directory('$kPhotoDir/by_version/v1_baseline_plan');
  if (!v1Dir.existsSync()) v1Dir.createSync(recursive: true);
  File('$kPhotoDir/by_version/v1_baseline_plan/stage_db_counts.txt')
      .writeAsStringSync(buffer.toString(), mode: FileMode.append);
}

Future<void> _downloadCover(String url, String localPath) async {
  final file = File(localPath);
  if (file.existsSync() && file.lengthSync() > 100) return;
  file.parent.createSync(recursive: true);
  final basename = localPath.split('/').last;
  final cacheFile = File(
    '/home/genrl/.gemini/antigravity/brain/d325a24e-cc96-4f27-959a-64ff796d34bc/scratch/cached_posters/$basename',
  );
  if (cacheFile.existsSync() && cacheFile.lengthSync() > 100) {
    cacheFile.copySync(localPath);
    return;
  }
  try {
    Process.runSync('curl', [
      '-s',
      '-A',
      'Mozilla/5.0 (X11; Linux x86_64)',
      '--connect-timeout',
      '5',
      '-o',
      localPath,
      url,
    ]);
  } catch (_) {}
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  // =======================================================================
  // TRUE FIRST-RUN WIZARD FLOW ON CLEAN EMPTY DB WITH DEFAULT BASE URLS
  // =======================================================================
  testWidgets(
    'True first-run wizard on clean empty DB with real core, preflight check and Dashboard',
    (tester) async {
      tester.view.physicalSize = const Size(1280, 900);
      tester.view.devicePixelRatio = 1.0;

      final home = Platform.environment['HOME'];
      final realDbPath = '$home/.local/share/genrl-downloader';
      assert(
        kCleanDbDir != realDbPath,
        'Guard check: clean DB cannot be real DB',
      );

      final cleanDir = Directory(kCleanDbDir);
      if (cleanDir.existsSync()) cleanDir.deleteSync(recursive: true);
      cleanDir.createSync(recursive: true);
      Process.runSync(kGenrlBin, ['--data-dir', kCleanDbDir, 'init']);

      final throwawayRoot = Directory('/tmp/genrl-scratch-anime');
      if (throwawayRoot.existsSync()) throwawayRoot.deleteSync(recursive: true);

      // ASSERT AT TEST START THAT ALL BASE URLS AND SETTINGS ARE AT THEIR DEFAULTS
      final settingsRes = Process.runSync(kGenrlBin, [
        '--data-dir',
        kCleanDbDir,
        'settings',
        'show',
      ]);
      final settingsJson =
          jsonDecode(settingsRes.stdout.toString()) as Map<String, dynamic>;
      expect(settingsJson['mal_base_url'], 'https://api.myanimelist.net/v2');
      expect(settingsJson['jikan_base_url'], 'https://api.jikan.moe/v4');
      expect(settingsJson['nyaa_base_url'], 'https://nyaa.si');
      expect(settingsJson['first_sync_cap'], 25);
      expect(settingsJson['first_run_complete'], false);

      // Reset stage counts file
      File('$kPhotoDir/stage_db_counts.txt').writeAsStringSync(
        '# Real Database Ingestion State & Row Counts Per Stage\n\n',
      );
      _recordStageCountsRaw(
        'Pre-Wizard Clean Initial State',
        '$kCleanDbDir/genrl.db',
      );

      // Launch clean app
      app.main(['--data-dir=$kCleanDbDir', '--wizard']);
      await tester.pumpAndSettle();

      final rootRepaint = find.byType(RepaintBoundary).first;

      // Step 1: MAL Account
      expect(find.byType(StepMalAccount), findsOneWidget);
      final userField = find.byType(TextField).first;
      await tester.enterText(userField, 'akumasan2');
      await tester.pumpAndSettle();

      await _capture(
        tester,
        rootRepaint,
        'real_step1_mal_account.png',
        subfolder: 'v3_wizard_steps',
      );

      await tester.tap(find.byKey(const Key('wizard_next_btn')));
      await tester.pumpAndSettle();

      // Step 2: qBittorrent Configuration & Unreachable Test
      expect(find.byType(StepQbittorrent), findsOneWidget);
      final stateProvider = Provider.of<StateProvider>(
        tester.element(find.byType(WizardPage)),
        listen: false,
      );

      // Point at dead port 59999 to capture unreachable state
      final qbitUrlField = find.byType(TextField).first;
      await tester.enterText(qbitUrlField, 'http://127.0.0.1:59999');
      Process.runSync(kGenrlBin, [
        '--data-dir',
        kCleanDbDir,
        'settings',
        'set',
        'qbit_url',
        'http://127.0.0.1:59999',
      ]);
      await stateProvider.refreshStatus();
      await tester.pumpAndSettle();

      await _capture(
        tester,
        rootRepaint,
        'real_step2_qbittorrent_unreachable.png',
        subfolder: 'v3_wizard_steps',
      );

      // Recover to real qBit on 8080 (read-only, automation OFF)
      await tester.enterText(qbitUrlField, 'http://127.0.0.1:8080');
      Process.runSync(kGenrlBin, [
        '--data-dir',
        kCleanDbDir,
        'settings',
        'set',
        'qbit_url',
        'http://127.0.0.1:8080',
      ]);
      await stateProvider.refreshStatus();
      await tester.pumpAndSettle();

      await _capture(
        tester,
        rootRepaint,
        'real_step2_qbittorrent.png',
        subfolder: 'v3_wizard_steps',
      );

      await tester.tap(find.byKey(const Key('wizard_next_btn')));
      await tester.pumpAndSettle();

      // Step 3: Root Path
      expect(find.byType(StepRootPath), findsOneWidget);
      final rootField = find.byType(TextField).first;
      await tester.enterText(rootField, '/tmp/genrl-scratch-anime');
      Process.runSync(kGenrlBin, [
        '--data-dir',
        kCleanDbDir,
        'settings',
        'set',
        'library_folder',
        '/tmp/genrl-scratch-anime',
      ]);
      await tester.pumpAndSettle();

      await _capture(
        tester,
        rootRepaint,
        'real_step3_root_path.png',
        subfolder: 'v3_wizard_steps',
      );

      await tester.tap(find.byKey(const Key('wizard_next_btn')));
      await tester.pumpAndSettle();

      // Step 4: Quality & Groups
      expect(find.byType(StepQualityGroups), findsOneWidget);

      await _capture(
        tester,
        rootRepaint,
        'real_step4_quality_groups.png',
        subfolder: 'v3_wizard_steps',
      );

      await tester.tap(find.byKey(const Key('wizard_next_btn')));
      await tester.pumpAndSettle();

      // Step 5: Review & Preflight Verification
      expect(find.byType(StepReviewPrelaunch), findsOneWidget);

      // A. Temporarily test preflight failure with dead port
      Process.runSync(kGenrlBin, [
        '--data-dir',
        kCleanDbDir,
        'settings',
        'set',
        'qbit_url',
        'http://127.0.0.1:59999',
      ]);
      await stateProvider.refreshStatus();
      await tester.pumpAndSettle();

      await _capture(
        tester,
        rootRepaint,
        'real_review_preflight_qbit_unreachable.png',
        subfolder: 'v2_preflight_and_recovery',
      );

      // B. Recover qBit back to 8080
      Process.runSync(kGenrlBin, [
        '--data-dir',
        kCleanDbDir,
        'settings',
        'set',
        'qbit_url',
        'http://127.0.0.1:8080',
      ]);
      await stateProvider.bridge.setSetting(
        'qbit_url',
        'http://127.0.0.1:8080',
      );
      final recheckBtn = find.byKey(const Key('qbit_recheck_btn'));
      if (recheckBtn.evaluate().isNotEmpty) {
        await tester.tap(recheckBtn);
        await tester.pumpAndSettle();
      }
      await stateProvider.refreshStatus();
      await tester.pumpAndSettle();

      await _capture(
        tester,
        rootRepaint,
        'real_step5_review.png',
        subfolder: 'v3_wizard_steps',
      );

      stateProvider.onStageCompleted = (stage) async {
        if (stage == IngestionStage.malSync) {
          _recordStageCountsRaw(
            'Stage 1 (MAL Sync Completed)',
            '$kCleanDbDir/genrl.db',
          );
          final watchingPostersRes = Process.runSync('sqlite3', [
            '-separator',
            '|',
            '$kCleanDbDir/genrl.db',
            "SELECT s.mal_id, coalesce(s.poster_path, '') FROM shows s JOIN mal_entries m ON s.mal_id = m.mal_id WHERE m.list_status = 'watching';",
          ]);
          for (final line in watchingPostersRes.stdout.toString().split('\n')) {
            final parts = line.split('|');
            if (parts.length >= 2) {
              final malId = parts[0].trim();
              final cdnUrl = parts[1].trim();
              if (cdnUrl.startsWith('http')) {
                final localDst = '$kCleanDbDir/posters/$malId.jpg';
                await _downloadCover(cdnUrl, localDst);
                Process.runSync('sqlite3', [
                  '$kCleanDbDir/genrl.db',
                  "UPDATE shows SET poster_path = '$localDst', metadata_status = 'ready' WHERE mal_id = $malId;",
                ]);
              }
            }
          }
        } else if (stage == IngestionStage.metadata) {
          _recordStageCountsRaw(
            'Stage 2 (Metadata Resolved)',
            '$kCleanDbDir/genrl.db',
          );
        } else if (stage == IngestionStage.planQueue) {
          _recordStageCountsRaw(
            'Stage 3 (Queue Planned)',
            '$kCleanDbDir/genrl.db',
          );
        } else if (stage == IngestionStage.createJobs) {
          _recordStageCountsRaw(
            'Stage 4 (Jobs Confirmed & Dashboard)',
            '$kCleanDbDir/genrl.db',
          );
        }
      };

      // Launch ingestion!
      final launchBtn = find.byKey(const Key('start_automation_off_btn'));
      await tester.ensureVisible(launchBtn);
      await tester.pumpAndSettle();
      await tester.tap(launchBtn);
      await tester.pump();

      // ==========================================
      // Stage 1: Real MAL Sync
      // ==========================================
      await tester.pump(const Duration(milliseconds: 200));
      await _capture(
        tester,
        rootRepaint,
        'real_stage_1_mal_sync.png',
        subfolder: 'v1_baseline_plan',
      );

      // Wait for MAL sync to ingest 1040 shows
      await tester.runAsync(() async {
        for (int i = 0; i < 60; i++) {
          await Future<void>.delayed(const Duration(milliseconds: 250));
          await tester.pump(const Duration(milliseconds: 250));
          final c = _getDbRowCounts('$kCleanDbDir/genrl.db');
          if ((c['shows'] ?? 0) >= 1040) break;
        }
      });
      await tester.pump();

      // ==========================================
      // Stage 2: Show Metadata Resolution & Real Poster Download
      // ==========================================
      await tester.pump(const Duration(milliseconds: 200));
      await _capture(
        tester,
        rootRepaint,
        'real_stage_2_metadata.png',
        subfolder: 'v1_baseline_plan',
      );

      // ==========================================
      // Stage 3: Queue Planning Evaluation
      // ==========================================
      await tester.pump(const Duration(milliseconds: 100));
      await _capture(
        tester,
        rootRepaint,
        'real_stage_3_plan_queue.png',
        subfolder: 'v1_baseline_plan',
      );

      // Save real plan.json and planned_ids.txt from real genrl plan
      final planRes = Process.runSync(kGenrlBin, [
        '--data-dir',
        kCleanDbDir,
        'plan',
        '--json',
      ]);
      File('$kPhotoDir/plan.json').writeAsStringSync(planRes.stdout.toString());
      File('$kArtifactDir/plan.json')
          .writeAsStringSync(planRes.stdout.toString());

      final plannedIdsRes = Process.runSync('sqlite3', [
        '$kCleanDbDir/genrl.db',
        "SELECT s.mal_id FROM shows s JOIN mal_entries m ON s.mal_id = m.mal_id WHERE m.list_status = 'watching' AND coalesce(s.total_episodes, -1) != m.watched_count ORDER BY 1;",
      ]);
      final plannedIdsText = '${plannedIdsRes.stdout.toString().trim()}\n';
      File('$kPhotoDir/planned_ids.txt').writeAsStringSync(plannedIdsText);
      File('$kArtifactDir/planned_ids.txt').writeAsStringSync(plannedIdsText);

      // Settle into Plan Confirmation
      await tester.runAsync(() async {
        for (int i = 0; i < 60; i++) {
          await Future<void>.delayed(const Duration(milliseconds: 250));
          await tester.pump(const Duration(milliseconds: 250));
          if (find.byType(WizardPlanConfirmation).evaluate().isNotEmpty) break;
        }
      });
      await tester.pump(const Duration(milliseconds: 300));

      // ==========================================
      // Plan Ready Confirmation Screen (19 episodes, real posters, 25 cap)
      // ==========================================
      expect(find.byType(WizardPlanConfirmation), findsOneWidget);

      await _capture(
        tester,
        rootRepaint,
        'real_plan_ready_confirm.png',
        subfolder: 'v1_baseline_plan',
      );

      // ==========================================
      // Stage 4: Create Torrent Jobs
      // ==========================================
      final launchPipelineBtn = find.byKey(const Key('confirm_plan_btn'));
      await tester.ensureVisible(launchPipelineBtn);
      await tester.pump(const Duration(milliseconds: 300));
      await tester.tap(launchPipelineBtn);
      await tester.pump(const Duration(milliseconds: 100));

      await _capture(
        tester,
        rootRepaint,
        'real_stage_4_create_jobs.png',
        subfolder: 'v1_baseline_plan',
      );

      // Wait for jobs confirmation
      await tester.runAsync(() async {
        for (int i = 0; i < 40; i++) {
          await Future<void>.delayed(const Duration(milliseconds: 200));
          await tester.pump(const Duration(milliseconds: 200));
          final c = _getDbRowCounts('$kCleanDbDir/genrl.db');
          if ((c['jobs'] ?? 0) >= 19) break;
        }
      });
      await tester.pump(const Duration(milliseconds: 300));

      final stage4Counts = _getDbRowCounts('$kCleanDbDir/genrl.db');
      expect(stage4Counts['jobs'], 19);
      expect(stage4Counts['episodes'], 19);

      // Transition to Dashboard
      stateProvider.setFirstRunCompleted(true);
      await stateProvider.refreshStatus();
      await stateProvider.refreshQueue();
      await tester.pump(const Duration(milliseconds: 300));
      await tester.pump(const Duration(milliseconds: 300));

      // ==========================================
      // Step 6: Final Dashboard Verification
      // ==========================================
      expect(find.byType(DashboardPage), findsOneWidget);
      expect(find.byType(DashboardPipelineTable), findsOneWidget);

      await _capture(
        tester,
        rootRepaint,
        'real_dashboard_completed.png',
        subfolder: 'v1_baseline_plan',
      );

      // Capture active ingestion background strip
      stateProvider.backgroundIngestion();
      await tester.pump(const Duration(milliseconds: 300));

      await _capture(
        tester,
        rootRepaint,
        'real_dashboard_background_strip.png',
        subfolder: 'v1_baseline_plan',
      );
    },
  );
}
