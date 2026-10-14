import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provider/provider.dart';
import 'package:genrl_downloader/pages/queue_page.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/theme/theme.dart';
import 'fake_core_bridge.dart';

Widget createTestApp(StateProvider stateProvider) {
  return ChangeNotifierProvider<StateProvider>.value(
    value: stateProvider,
    child: MaterialApp(
      theme: ObsidianTheme.darkTheme,
      home: const Scaffold(body: QueuePage()),
    ),
  );
}

void main() {
  setUp(() {
    TestWidgetsFlutterBinding.ensureInitialized();
  });

  testWidgets('Queue Page renders active torrent transfers and filter tabs', (tester) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final fakeBridge = FakeCoreBridge(mode: FakeMode.data);
    final state = StateProvider(bridge: fakeBridge, startPolling: false);
    await state.refreshAll();

    await tester.pumpWidget(createTestApp(state));
    await tester.pump();

    // Verify filter tabs
    expect(find.text('All (2)'), findsOneWidget);
    expect(find.text('Downloading (1)'), findsOneWidget);
    expect(find.text('Pause Queue'), findsOneWidget);

    // Verify torrent items rendered
    expect(find.text('Sousou no Frieren'), findsAtLeastNWidgets(1));
    expect(find.text('Dungeon Meshi'), findsOneWidget);
    expect(find.textContaining('142 seeds'), findsOneWidget);
  });

  testWidgets('Queue Page renders empty state when no transfers exist', (tester) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final fakeBridge = FakeCoreBridge(mode: FakeMode.empty);
    final state = StateProvider(bridge: fakeBridge, startPolling: false);
    await state.refreshAll();

    await tester.pumpWidget(createTestApp(state));
    await tester.pump();

    expect(find.text('No Transfers In Queue'), findsOneWidget);
    expect(find.textContaining('Active downloads and jobs will appear here live'), findsOneWidget);
  });

  testWidgets('Queue Page displays error message and Retry button on error', (tester) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final fakeBridge = FakeCoreBridge(
      mode: FakeMode.error,
      errorMessage: 'qBittorrent WebUI unreachable on 127.0.0.1:8080',
    );
    final state = StateProvider(bridge: fakeBridge, startPolling: false);
    await state.refreshAll();

    await tester.pumpWidget(createTestApp(state));
    await tester.pump();

    expect(find.text('Queue Query Error'), findsOneWidget);
    expect(find.textContaining('qBittorrent WebUI unreachable'), findsOneWidget);
    expect(find.text('Retry Queue'), findsOneWidget);

    // Switch to data and tap Retry
    fakeBridge.mode = FakeMode.data;
    await tester.tap(find.text('Retry Queue'));
    await tester.pump();

    expect(find.text('Sousou no Frieren'), findsAtLeastNWidgets(1));
  });
}
