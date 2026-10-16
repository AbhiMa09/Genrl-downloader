import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provider/provider.dart';
import 'package:genrl_downloader/pages/rules_page.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/theme/theme.dart';

import 'fake_core_bridge.dart';

Widget createTestApp(StateProvider stateProvider) {
  return ChangeNotifierProvider<StateProvider>.value(
    value: stateProvider,
    child: MaterialApp(
      theme: ObsidianTheme.darkTheme,
      home: const Scaffold(body: RulesPage()),
    ),
  );
}

void main() {
  setUp(() {
    TestWidgetsFlutterBinding.ensureInitialized();
  });

  testWidgets('Rules Page renders editable global rules and controls', (
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

    expect(find.text('Rules & Automation Policies'), findsOneWidget);
    expect(find.text('LIVE ENGINE'), findsOneWidget);
    expect(find.text('Save Changes'), findsOneWidget);
    expect(find.text('Test RSS Feed'), findsOneWidget);

    // Save rules action
    await tester.tap(find.text('Save Changes'));
    await tester.pump();
  });

  testWidgets('Rules Page displays error message and Retry button on error', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final fakeBridge = FakeCoreBridge(
      mode: FakeMode.error,
      errorMessage: 'Failed to read rules from SQLite config',
    );
    final state = StateProvider(bridge: fakeBridge, startPolling: false);
    await state.refreshAll();

    await tester.pumpWidget(createTestApp(state));
    await tester.pump();

    expect(find.text('Rules Fetch Error'), findsOneWidget);
    expect(
      find.textContaining('Failed to read rules from SQLite config'),
      findsOneWidget,
    );
    expect(find.text('Retry Rules Query'), findsOneWidget);

    // Switch to data and tap Retry
    fakeBridge.mode = FakeMode.data;
    await tester.tap(find.text('Retry Rules Query'));
    await tester.pump();

    expect(find.text('Rules & Automation Policies'), findsOneWidget);
  });
}
