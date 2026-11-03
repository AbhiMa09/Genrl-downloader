import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provider/provider.dart';
import 'package:genrl_downloader/models/queue_item.dart';
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

  testWidgets('Queue Page renders active torrent transfers and filter tabs', (
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

    // Verify filter tabs
    expect(find.text('All (2)'), findsOneWidget);
    expect(find.text('Downloading (1)'), findsOneWidget);
    expect(find.text('Pause Queue'), findsOneWidget);

    // Verify torrent items rendered
    expect(find.text('Sousou no Frieren'), findsAtLeastNWidgets(1));
    expect(find.text('Dungeon Meshi'), findsOneWidget);
    expect(find.textContaining('142 seeds'), findsAtLeastNWidgets(1));
  });

  testWidgets('Queue Page renders empty state when no transfers exist', (
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

    expect(find.text('No Transfers In Queue'), findsOneWidget);
    expect(
      find.textContaining('Active downloads and jobs will appear here live'),
      findsOneWidget,
    );
  });

  testWidgets('Queue Page displays error message and Retry button on error', (
    tester,
  ) async {
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
    expect(
      find.textContaining('qBittorrent WebUI unreachable'),
      findsOneWidget,
    );
    expect(find.text('Retry Queue'), findsOneWidget);

    // Switch to data and tap Retry
    fakeBridge.mode = FakeMode.data;
    await tester.tap(find.text('Retry Queue'));
    await tester.pump();

    expect(find.text('Sousou no Frieren'), findsAtLeastNWidgets(1));
  });

  testWidgets('Queue Page displays failed job with error and allows retry', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final fakeBridge = FakeCoreBridge(
      customQueueItems: [
        const QueueItem(
          id: 42,
          malId: 18119,
          showTitle: 'Servant x Service',
          season: 1,
          episode: 1,
          releaseName: '[neoHEVC] Servant x Service - 01',
          group: 'neoHEVC',
          progress: 0.0,
          speedBytes: 0,
          seeders: 0,
          leechers: 0,
          sizeBytes: 250000000,
          etaSeconds: 0,
          state: QueueState.failed,
          error: 'No healthy torrent seeds available',
        ),
      ],
    );
    final state = StateProvider(bridge: fakeBridge, startPolling: false);
    await state.refreshAll();

    await tester.pumpWidget(createTestApp(state));
    await tester.pump();

    // Verify filter shows Failed (1)
    expect(find.text('Failed (1)'), findsOneWidget);

    // Verify job title, error, and Failed status
    expect(find.text('Servant x Service'), findsAtLeastNWidgets(1));
    expect(
      find.text('No healthy torrent seeds available'),
      findsAtLeastNWidgets(1),
    );
    expect(find.text('Failed'), findsAtLeastNWidgets(1));

    // Tap the 'Failed (1)' filter tab
    await tester.tap(find.text('Failed (1)'));
    await tester.pump();
    expect(find.text('Servant x Service'), findsAtLeastNWidgets(1));

    // Find and tap the Retry Job icon button
    final retryFinder = find.byTooltip('Retry Job');
    expect(retryFinder, findsOneWidget);
    await tester.tap(retryFinder);
    await tester.pump();

    expect(fakeBridge.executed, contains('job retry 42'));
    expect(find.text('Retried job #42'), findsOneWidget);
  });
}
