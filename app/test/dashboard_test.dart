import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provider/provider.dart';
import 'package:genrl_downloader/pages/dashboard_page.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/theme/theme.dart';
import 'fake_core_bridge.dart';

Widget createTestApp(StateProvider stateProvider) {
  return ChangeNotifierProvider<StateProvider>.value(
    value: stateProvider,
    child: MaterialApp(
      theme: ObsidianTheme.darkTheme,
      home: const Scaffold(body: DashboardPage()),
    ),
  );
}

void main() {
  setUp(() {
    TestWidgetsFlutterBinding.ensureInitialized();
  });

  testWidgets('Dashboard Page renders populated data correctly', (tester) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final fakeBridge = FakeCoreBridge(mode: FakeMode.data);
    final state = StateProvider(bridge: fakeBridge, startPolling: false);
    await state.refreshAll();

    await tester.pumpWidget(createTestApp(state));
    await tester.pump();

    // Verify system telemetry & cards
    expect(find.text('qBittorrent v4.6.5 :: IPC Linked'), findsOneWidget);
    expect(find.text('3 Active Torrents'), findsOneWidget);
    expect(find.text('12 Monitored Shows'), findsOneWidget);
    expect(find.text('ACTIVE DOWNLOAD TRANSFERS'), findsOneWidget);
    expect(find.text('Sousou no Frieren'), findsWidgets);
  });

  testWidgets('Dashboard Page renders empty state gracefully', (tester) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final fakeBridge = FakeCoreBridge(mode: FakeMode.empty);
    final state = StateProvider(bridge: fakeBridge, startPolling: false);
    await state.refreshAll();

    await tester.pumpWidget(createTestApp(state));
    await tester.pump();

    expect(find.text('Queue is empty. Automation is watching for newly aired releases.'), findsOneWidget);
    expect(find.text('No organized shows discovered yet. Run a disk scan or connect MAL to populate.'), findsOneWidget);
  });

  testWidgets('Dashboard Page displays real error and Retry button on core error', (tester) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final fakeBridge = FakeCoreBridge(
      mode: FakeMode.error,
      errorMessage: 'SocketException: Connection refused on 127.0.0.1:21445',
    );
    final state = StateProvider(bridge: fakeBridge, startPolling: false);
    await state.refreshAll();

    await tester.pumpWidget(createTestApp(state));
    await tester.pump();

    expect(find.text('Core Engine Unreachable'), findsOneWidget);
    expect(find.textContaining('SocketException: Connection refused'), findsOneWidget);
    expect(find.text('Retry Connection'), findsOneWidget);

    // Switch mode to data and tap Retry
    fakeBridge.mode = FakeMode.data;
    await tester.tap(find.text('Retry Connection'));
    await tester.pump();

    // Verify recovery after retry
    expect(find.text('ACTIVE DOWNLOAD TRANSFERS'), findsOneWidget);
  });
}
