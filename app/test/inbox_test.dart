import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provider/provider.dart';
import 'package:genrl_downloader/pages/inbox_page.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/theme/theme.dart';

import 'fake_core_bridge.dart';

Widget createTestApp(StateProvider stateProvider) {
  return ChangeNotifierProvider<StateProvider>.value(
    value: stateProvider,
    child: MaterialApp(
      theme: ObsidianTheme.darkTheme,
      home: const Scaffold(body: InboxPage()),
    ),
  );
}

void main() {
  setUp(() {
    TestWidgetsFlutterBinding.ensureInitialized();
  });

  testWidgets('Inbox Page renders question card and choice actions', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final fakeBridge = FakeCoreBridge(mode: FakeMode.data);
    final state = StateProvider(bridge: fakeBridge, startPolling: false);
    await state.refreshAll();

    await tester.pumpWidget(createTestApp(state));
    await tester.pump();

    expect(find.text('Action Required (1)'), findsOneWidget);
    expect(find.text('Existing File Collision: Frieren EP 12'), findsOneWidget);
    expect(find.text('Keep Existing'), findsOneWidget);
    expect(find.text('Replace File'), findsOneWidget);

    // Tap choice action
    await tester.tap(find.text('Keep Existing'));
    await tester.pump();
  });

  testWidgets('Inbox Page renders empty state when no questions remain', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final fakeBridge = FakeCoreBridge(mode: FakeMode.empty);
    final state = StateProvider(bridge: fakeBridge, startPolling: false);
    await state.refreshAll();

    await tester.pumpWidget(createTestApp(state));
    await tester.pump();

    expect(find.text('All Caught Up!'), findsOneWidget);
    expect(
      find.textContaining('No pending questions in your Inbox'),
      findsOneWidget,
    );
  });

  testWidgets('Inbox Page displays error message and Retry button on error', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final fakeBridge = FakeCoreBridge(
      mode: FakeMode.error,
      errorMessage: 'Database disk I/O failure',
    );
    final state = StateProvider(bridge: fakeBridge, startPolling: false);
    await state.refreshAll();

    await tester.pumpWidget(createTestApp(state));
    await tester.pump();

    expect(find.text('Inbox Query Error'), findsOneWidget);
    expect(find.textContaining('Database disk I/O failure'), findsOneWidget);
    expect(find.text('Retry Inbox'), findsOneWidget);

    // Switch mode to data and tap Retry
    fakeBridge.mode = FakeMode.data;
    await tester.tap(find.text('Retry Inbox'));
    await tester.pump();

    expect(find.text('Existing File Collision: Frieren EP 12'), findsOneWidget);
  });
}
