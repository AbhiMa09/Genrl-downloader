import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:google_fonts/google_fonts.dart';
import 'package:genrl_downloader/services/core_bridge.dart';
import 'package:genrl_downloader/services/mal_auth_error.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/pages/settings/mal_connection_controls.dart';

import 'test_font_helper.dart';

class AuthFixtureBridge extends CoreBridge {
  AuthFixtureBridge(String dir) : super(customDataDir: dir);
  final pending = Completer<ProcessResult>();
  int starts = 0;
  List<String>? arguments;
  Map<String, String>? childEnvironment;
  @override
  Future<Map<String, dynamic>?> getSettings() async => {
    'mal_client_id': 'fixture-client',
  };
  @override
  Future<Map<String, dynamic>?> getMalStatus() async => {
    'status': 'public_profile',
  };
  @override
  Future<ProcessResult> execute(
    List<String> args, {
    Map<String, String>? environment,
  }) {
    if (args.length < 2 || args[0] != 'mal' || args[1] != 'connect') {
      return Future.value(ProcessResult(0, 0, '{}', ''));
    }
    starts++;
    arguments = args;
    childEnvironment = environment;
    return pending.future;
  }
}

void main() {
  late Directory scratch;
  late AuthFixtureBridge bridge;
  late StateProvider state;
  setUpAll(() async {
    GoogleFonts.config.allowRuntimeFetching = false;
    await loadAppFonts();
  });
  setUp(() {
    scratch = Directory.systemTemp.createTempSync('genrl-auth-fixture-');
    bridge = AuthFixtureBridge(scratch.path);
    state = StateProvider(bridge: bridge, startPolling: false);
  });
  tearDown(() {
    state.dispose();
    scratch.deleteSync();
  });

  test('errors disclose only the failure class', () {
    for (final raw in [
      'invalid_client FIXTURE_SECRET http://localhost/callback?code=FIXTURE_CODE',
      'invalid_grant FIXTURE_SECRET',
      'HTTP 503 FIXTURE_SECRET',
      'unrecognized FIXTURE_SECRET',
    ]) {
      final result = malAuthErrorSummary(raw);
      expect(result, isNot(contains('FIXTURE')));
      expect(result, isNot(contains('callback?')));
    }
  });

  test(
    'pending authorization blocks duplicates and secret stays out of argv',
    () async {
      final task = state.connectMal(
        clientId: 'fixture-client',
        clientSecret: 'fixture-secret',
      );
      expect(state.isConnectingMal, isTrue);
      await state.connectMal();
      expect(bridge.starts, 1);
      expect(bridge.arguments!.join(' '), isNot(contains('fixture-secret')));
      expect(bridge.arguments, isNot(contains('--client-secret')));
      expect(
        bridge.childEnvironment!['GENRL_MAL_CLIENT_SECRET'],
        'fixture-secret',
      );
      bridge.pending.complete(
        ProcessResult(0, 1, '', 'invalid_client FIXTURE_SECRET'),
      );
      await task;
      expect(state.isConnectingMal, isFalse);
      expect(state.malError, contains('MAL rejected the client'));
      expect(state.malError, isNot(contains('FIXTURE_SECRET')));
    },
  );

  testWidgets('connection dialog masks secret and renders failed result', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: MalConnectionControls(state: state)),
      ),
    );
    await tester.tap(find.byKey(const Key('mal_connect_browser')));
    await tester.pumpAndSettle();
    final fields = tester
        .widgetList<TextField>(find.byType(TextField))
        .toList();
    expect(fields.first.controller!.text, 'fixture-client');
    expect(fields.last.obscureText, isTrue);
    await tester.enterText(
      find.byKey(const Key('mal_client_secret')),
      'fixture-secret',
    );
    await tester.tap(find.byKey(const Key('mal_authorize')));
    await tester.pumpAndSettle();
    expect(find.text('Waiting for authorization…'), findsOneWidget);
    await tester.runAsync(() async {
      bridge.pending.complete(
        ProcessResult(0, 1, '', 'invalid_grant FIXTURE_SECRET'),
      );
      await bridge.pending.future;
      await Future<void>.delayed(Duration.zero);
    });
    await tester.pumpAndSettle();
    await tester.pump();
    expect(state.malError, contains('Start a fresh browser login'));
    expect(find.textContaining('Start a fresh browser login'), findsOneWidget);
    expect(find.textContaining('FIXTURE_SECRET'), findsNothing);
  });
}
