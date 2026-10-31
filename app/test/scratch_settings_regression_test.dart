// All endpoints, settings and status values here are fixtures. No CLI is launched.
import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:google_fonts/google_fonts.dart';
import 'package:provider/provider.dart';
import 'package:genrl_downloader/models/library_item.dart';
import 'package:genrl_downloader/models/system_status.dart';
import 'package:genrl_downloader/pages/settings/settings_page.dart';
import 'package:genrl_downloader/pages/wizard/wizard_page.dart';
import 'package:genrl_downloader/pages/wizard/step_qbittorrent.dart';
import 'package:genrl_downloader/pages/wizard/step_root_path.dart';
import 'package:genrl_downloader/services/core_bridge.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/theme/theme.dart';

import 'test_font_helper.dart';

class FixtureBridge extends CoreBridge {
  FixtureBridge(String scratch) : super(customDataDir: scratch);
  Completer<void>? loading;
  Completer<bool>? saving;
  bool failSave = false;
  String? attemptedUrl;
  final values = <String, dynamic>{'qbit_url': 'http://127.0.0.1:18080'};

  @override
  Future<ProcessResult> execute(
    List<String> args, {
    Map<String, String>? environment,
  }) async => ProcessResult(0, 0, '{}', '');

  @override
  Future<Map<String, dynamic>?> getSettings() async {
    await loading?.future;
    return Map<String, dynamic>.from(values);
  }

  @override
  Future<bool> setSetting(String key, dynamic value) async {
    if (key == 'qbit_url') attemptedUrl = value as String;
    final ok = saving != null ? await saving!.future : !failSave;
    if (ok) values[key] = value;
    return ok;
  }

  @override
  Future<SystemStatus?> getSystemStatus() async => const SystemStatus(
    daemonConnected: true,
    qbitConnected: true,
    qbitVersion: 'fixture-version',
  );
}

void main() {
  late Directory scratch;
  late FixtureBridge bridge;
  late StateProvider state;

  setUpAll(() async {
    GoogleFonts.config.allowRuntimeFetching = false;
    await loadAppFonts();
  });
  setUp(() {
    scratch = Directory.systemTemp.createTempSync('genrl-widget-fixture-');
    bridge = FixtureBridge(scratch.path);
    state = StateProvider(bridge: bridge, startPolling: false);
  });
  tearDown(() {
    state.dispose();
    scratch.deleteSync();
  });

  Future<void> show(WidgetTester tester, Widget page) async {
    tester.view.physicalSize = const Size(1600, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(() => tester.view.resetPhysicalSize());
    await tester.pumpWidget(
      ChangeNotifierProvider<StateProvider>.value(
        value: state,
        child: MaterialApp(
          theme: ObsidianTheme.darkTheme,
          home: Scaffold(body: page),
        ),
      ),
    );
    await tester.pump();
  }

  testWidgets(
    'saved qBit endpoint loads and navigation waits for persistence',
    (tester) async {
      bridge.loading = Completer<void>();
      await show(tester, WizardPage(initialStep: 1, onComplete: () {}));
      var next = tester.widget<ElevatedButton>(
        find.byKey(const Key('wizard_next_btn')),
      );
      expect(next.onPressed, isNull);
      bridge.loading!.complete();
      await tester.pumpAndSettle();
      final endpoint = tester.widget<TextField>(find.byType(TextField).first);
      expect(endpoint.controller!.text, 'http://127.0.0.1:18080');

      bridge.saving = Completer<bool>();
      await tester.ensureVisible(find.byKey(const Key('wizard_next_btn')));
      await tester.tap(find.byKey(const Key('wizard_next_btn')));
      await tester.pump();
      expect(find.byType(StepQbittorrent), findsOneWidget);
      expect(bridge.attemptedUrl, 'http://127.0.0.1:18080');
      next = tester.widget<ElevatedButton>(
        find.byKey(const Key('wizard_next_btn')),
      );
      expect(next.onPressed, isNull);
      bridge.saving!.complete(true);
      await tester.pumpAndSettle();
      expect(find.byType(StepRootPath), findsOneWidget);
      expect(bridge.values['qbit_url'], 'http://127.0.0.1:18080');
    },
  );

  testWidgets('failed endpoint persistence keeps the connection step open', (
    tester,
  ) async {
    bridge.failSave = true;
    await show(tester, WizardPage(initialStep: 1, onComplete: () {}));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.byKey(const Key('wizard_next_btn')));
    await tester.tap(find.byKey(const Key('wizard_next_btn')));
    await tester.pumpAndSettle();
    expect(find.byType(StepQbittorrent), findsOneWidget);
    expect(
      find.textContaining('Could not save qBittorrent endpoint'),
      findsOneWidget,
    );
  });

  testWidgets(
    'late settings do not overwrite an endpoint already being edited',
    (tester) async {
      bridge.loading = Completer<void>();
      await show(tester, WizardPage(initialStep: 1, onComplete: () {}));
      await tester.enterText(
        find.byType(TextField).first,
        'http://localhost:18080',
      );
      bridge.loading!.complete();
      await tester.pumpAndSettle();
      expect(
        tester.widget<TextField>(find.byType(TextField).first).controller!.text,
        'http://localhost:18080',
      );
    },
  );

  testWidgets(
    'settings reopening shows the saved endpoint without a fake password',
    (tester) async {
      await show(tester, const SettingsPage());
      await tester.pumpAndSettle();
      final fields = tester
          .widgetList<TextField>(find.byType(TextField))
          .toList();
      expect(
        fields.any((f) => f.controller?.text == 'http://127.0.0.1:18080'),
        isTrue,
      );
      expect(
        fields
            .where((f) => f.obscureText)
            .every((f) => f.controller!.text.isEmpty),
        isTrue,
      );
    },
  );

  test('only ready and partial metadata can be downloaded', () {
    const fixture = LibraryShow(
      malId: 1,
      title: 'Fixture',
      startYear: 2024,
      totalEpisodes: 12,
      downloadedEpisodes: 0,
      watchedEpisodes: 0,
      score: 0,
      status: 'Watching',
      airingStatus: 'Airing',
      genres: [],
      synopsis: '',
    );
    for (final status in ['pending', 'fetching', 'failed']) {
      expect(
        fixture.copyWith(metadataStatus: status).metadataResolved,
        isFalse,
      );
    }
    expect(fixture.copyWith(metadataStatus: 'ready').metadataResolved, isTrue);
    expect(
      fixture.copyWith(metadataStatus: 'partial').metadataResolved,
      isTrue,
    );
  });
}
