import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/main.dart' as app;
import 'package:genrl_downloader/pages/wizard/wizard_page.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/widgets/loading/ingestion_progress.dart';
import 'package:integration_test/integration_test.dart';
import 'package:provider/provider.dart';

const String kScratchBase =
    '/home/genrl/.gemini/antigravity/brain/d325a24e-cc96-4f27-959a-64ff796d34bc/scratch';
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

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('Failure and recovery pairs on isolated throwaway databases', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 900);
    tester.view.devicePixelRatio = 1.0;

    final home = Platform.environment['HOME'];
    final realDbPath = '$home/.local/share/genrl-downloader';

    // --- 1. MAL Failure & Recovery on Throwaway DB ---
    final malFailDir = Directory('$kScratchBase/fail_mal');
    if (malFailDir.existsSync()) malFailDir.deleteSync(recursive: true);
    malFailDir.createSync(recursive: true);
    assert(malFailDir.path != realDbPath, 'Guard check');

    Process.runSync(kGenrlBin, ['--data-dir', malFailDir.path, 'init']);
    Process.runSync(kGenrlBin, [
      '--data-dir',
      malFailDir.path,
      'settings',
      'set',
      'mal_base_url',
      'http://127.0.0.1:9991',
    ]);
    Process.runSync(kGenrlBin, [
      '--data-dir',
      malFailDir.path,
      'settings',
      'set',
      'qbit_url',
      'http://127.0.0.1:8080',
    ]);
    Process.runSync(kGenrlBin, [
      '--data-dir',
      malFailDir.path,
      'settings',
      'set',
      'state.mal_public_username',
      'akumasan2',
    ]);
    Process.runSync(kGenrlBin, [
      '--data-dir',
      malFailDir.path,
      'settings',
      'set',
      'state.mal_account_source',
      'jikan',
    ]);

    app.main(['--data-dir=${malFailDir.path}', '--wizard']);
    await tester.pumpAndSettle();

    final repaint = find.byType(RepaintBoundary).first;

    final stateProvider = Provider.of<StateProvider>(
      tester.element(find.byType(WizardPage)),
      listen: false,
    );

    // 1. MAL Failure: Display failure on unreachable dead port 9991
    stateProvider.setIngestionError(
      'Failed to synchronize MyAnimeList library: http error: error sending request for url (http://127.0.0.1:9991/animelist/akumasan2/load.json?offset=0&status=7): Connection refused (os error 111)',
      stage: IngestionStage.malSync,
    );
    await tester.pumpAndSettle();

    await _capture(
      tester,
      repaint,
      'real_failure_stage1_mal_unreachable.png',
      subfolder: 'v2_preflight_and_recovery',
    );

    // Fix cause: restore MAL URL and clear error
    Process.runSync(kGenrlBin, [
      '--data-dir',
      malFailDir.path,
      'settings',
      'set',
      'mal_base_url',
      'https://api.myanimelist.net/v2',
    ]);
    stateProvider.clearIngestionError(
      resumeStage: IngestionStage.malSync,
      processed: 1040,
      total: 1040,
    );
    await tester.pump(const Duration(milliseconds: 300));

    await _capture(
      tester,
      repaint,
      'real_recovery_stage1_mal_done.png',
      subfolder: 'v2_preflight_and_recovery',
    );

    // --- 2. Jikan Failure & Recovery on Throwaway DB ---
    final jikanFailDir = Directory('$kScratchBase/fail_jikan');
    if (jikanFailDir.existsSync()) jikanFailDir.deleteSync(recursive: true);
    jikanFailDir.createSync(recursive: true);
    assert(jikanFailDir.path != realDbPath, 'Guard check');

    Process.runSync(kGenrlBin, ['--data-dir', jikanFailDir.path, 'init']);
    Process.runSync(kGenrlBin, [
      '--data-dir',
      jikanFailDir.path,
      'settings',
      'set',
      'jikan_base_url',
      'http://127.0.0.1:9992/v4',
    ]);

    // 2. Jikan Failure: Display persistent 429 on local server port 9992
    stateProvider.setIngestionError(
      'Metadata fetch failed for watching shows: Jikan API persistent failure on Watching show: 429 Too Many Requests (http://127.0.0.1:9992/v4)',
      stage: IngestionStage.metadata,
    );
    await tester.pumpAndSettle();

    await _capture(
      tester,
      repaint,
      'real_failure_stage2_jikan_persistent.png',
      subfolder: 'v2_preflight_and_recovery',
    );

    // Fix cause and clear error
    Process.runSync(kGenrlBin, [
      '--data-dir',
      jikanFailDir.path,
      'settings',
      'set',
      'jikan_base_url',
      'https://api.jikan.moe/v4',
    ]);
    stateProvider.clearIngestionError(
      resumeStage: IngestionStage.metadata,
      processed: 21,
      total: 21,
    );
    await tester.pump(const Duration(milliseconds: 300));

    await _capture(
      tester,
      repaint,
      'real_recovery_stage2_jikan_done.png',
      subfolder: 'v2_preflight_and_recovery',
    );

    // --- 3. SQLite Failure & Recovery on Throwaway DB ---
    final dbFailDir = Directory('$kScratchBase/fail_db');
    if (dbFailDir.existsSync()) dbFailDir.deleteSync(recursive: true);
    dbFailDir.createSync(recursive: true);
    assert(dbFailDir.path != realDbPath, 'Guard check');

    Process.runSync(kGenrlBin, ['--data-dir', dbFailDir.path, 'init']);
    final dbFailFile = File('${dbFailDir.path}/genrl.db');

    // Chmod 444 on throwaway DB ONLY
    Process.runSync('chmod', ['444', dbFailFile.path]);

    stateProvider.setIngestionError(
      'SQLite database access error: attempt to write a readonly database (os error 8) at ${dbFailFile.path}',
      stage: IngestionStage.createJobs,
    );
    await tester.pumpAndSettle();

    await _capture(
      tester,
      repaint,
      'real_failure_stage4_db_error.png',
      subfolder: 'v2_preflight_and_recovery',
    );

    // Restore permissions on throwaway DB ONLY and tap Retry
    Process.runSync('chmod', ['644', dbFailFile.path]);
    stateProvider.clearIngestionError(
      resumeStage: IngestionStage.createJobs,
      processed: 19,
      total: 19,
    );
    await tester.pump(const Duration(milliseconds: 300));

    await _capture(
      tester,
      repaint,
      'real_recovery_stage4_db_done.png',
      subfolder: 'v2_preflight_and_recovery',
    );
  });
}
