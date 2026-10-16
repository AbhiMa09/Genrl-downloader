import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provider/provider.dart';
import 'package:genrl_downloader/pages/library_page.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/theme/theme.dart';

import 'fake_core_bridge.dart';

Widget createTestApp(StateProvider stateProvider) {
  return ChangeNotifierProvider<StateProvider>.value(
    value: stateProvider,
    child: MaterialApp(
      theme: ObsidianTheme.darkTheme,
      home: const Scaffold(body: LibraryPage()),
    ),
  );
}

void main() {
  setUp(() {
    TestWidgetsFlutterBinding.ensureInitialized();
  });

  testWidgets('Library Page renders populated shows list correctly', (
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

    expect(find.text('Anime Library'), findsOneWidget);
    expect(find.text('Sousou no Frieren'), findsOneWidget);
    expect(find.text('Dungeon Meshi'), findsOneWidget);
    expect(find.text('Watching (1)'), findsOneWidget);
    expect(find.text('Plan to Watch (1)'), findsOneWidget);
  });

  testWidgets('Library Page renders empty / not connected state', (
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

    expect(find.text('MyAnimeList Not Connected'), findsOneWidget);
    expect(find.text('Connect MAL in Settings'), findsOneWidget);
  });

  testWidgets(
    'Library Page displays real error message and Retry button on error',
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() => tester.view.resetPhysicalSize());

      final fakeBridge = FakeCoreBridge(
        mode: FakeMode.error,
        errorMessage: 'Database locked by concurrent worker',
      );
      final state = StateProvider(bridge: fakeBridge, startPolling: false);
      await state.refreshAll();

      await tester.pumpWidget(createTestApp(state));
      await tester.pump();

      expect(find.text('Library Sync Error'), findsOneWidget);
      expect(
        find.textContaining('Database locked by concurrent worker'),
        findsOneWidget,
      );
      expect(find.text('Retry Library Query'), findsOneWidget);

      // Switch back to data and tap retry
      fakeBridge.mode = FakeMode.data;
      await tester.tap(find.text('Retry Library Query'));
      await tester.pump();

      expect(find.text('Sousou no Frieren'), findsOneWidget);
    },
  );
}
