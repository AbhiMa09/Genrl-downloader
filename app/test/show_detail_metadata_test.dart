import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provider/provider.dart';
import 'package:genrl_downloader/models/library_item.dart';
import 'package:genrl_downloader/pages/show_detail_page.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/theme/theme.dart';

import 'fake_core_bridge.dart';

Widget createTestApp(StateProvider stateProvider, LibraryShow show) {
  return ChangeNotifierProvider<StateProvider>.value(
    value: stateProvider,
    child: MaterialApp(
      theme: ObsidianTheme.darkTheme,
      home: ShowDetailPage(show: show),
    ),
  );
}

void main() {
  setUp(() {
    TestWidgetsFlutterBinding.ensureInitialized();
  });

  testWidgets(
    'Show Detail Page displays loading state when metadataStatus is fetching',
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() => tester.view.resetPhysicalSize());

      const testShow = LibraryShow(
        malId: 31319,
        title: '12-sai. 2nd Season',
        startYear: 2016,
        totalEpisodes: 12,
        downloadedEpisodes: 0,
        watchedEpisodes: 0,
        score: 6.8,
        status: 'Plan to Watch',
        airingStatus: 'Finished',
        genres: ['Romance', 'School'],
        synopsis: '[Synopsis loading...]',
        metadataStatus: 'fetching',
      );

      final fakeBridge = FakeCoreBridge(
        mode: FakeMode.data,
        customShowDetail: testShow,
      );
      final state = StateProvider(bridge: fakeBridge, startPolling: false);

      await tester.pumpWidget(createTestApp(state, testShow));
      await tester.pump();

      expect(find.text('Synopsis loading...'), findsOneWidget);
      expect(find.text('Refreshing...'), findsOneWidget);
    },
  );

  testWidgets(
    'Show Detail Page displays retry banner and button when metadataStatus is pending',
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() => tester.view.resetPhysicalSize());

      const testShow = LibraryShow(
        malId: 31319,
        title: '12-sai. 2nd Season',
        startYear: 2016,
        totalEpisodes: 12,
        downloadedEpisodes: 0,
        watchedEpisodes: 0,
        score: 6.8,
        status: 'Plan to Watch',
        airingStatus: 'Finished',
        genres: ['Romance', 'School'],
        synopsis:
            '[Synopsis unavailable: upstream rate limit. Will retry shortly.]',
        metadataStatus: 'pending',
        metadataError: 'rate limit 429',
        metadataNextRetryAt: 1760000060,
      );

      final fakeBridge = FakeCoreBridge(
        mode: FakeMode.data,
        customShowDetail: testShow,
      );
      final state = StateProvider(bridge: fakeBridge, startPolling: false);

      await tester.pumpWidget(createTestApp(state, testShow));
      await tester.pump();

      expect(find.textContaining('Could not load, retrying'), findsOneWidget);
      expect(find.text('Retry'), findsOneWidget);

      // Tap retry button
      await tester.tap(find.text('Retry'));
      await tester.pump();
    },
  );

  testWidgets(
    'Show Detail Page displays plain text synopsis and Refresh Metadata button when ready',
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() => tester.view.resetPhysicalSize());

      const testShow = LibraryShow(
        malId: 31319,
        title: '12-sai. 2nd Season',
        startYear: 2016,
        totalEpisodes: 12,
        downloadedEpisodes: 0,
        watchedEpisodes: 0,
        score: 6.8,
        status: 'Plan to Watch',
        airingStatus: 'Finished',
        genres: ['Romance', 'School'],
        synopsis: 'Hanabi Ayase and Yui Aoi are sixth-graders facing the trials and tribulations of first love.',
        metadataStatus: 'ready',
      );

      final fakeBridge = FakeCoreBridge(
        mode: FakeMode.data,
        customShowDetail: testShow,
      );
      final state = StateProvider(bridge: fakeBridge, startPolling: false);

      await tester.pumpWidget(createTestApp(state, testShow));
      await tester.pump();

      expect(
        find.text(
          'Hanabi Ayase and Yui Aoi are sixth-graders facing the trials and tribulations of first love.',
        ),
        findsOneWidget,
      );
      expect(find.text('Refresh Metadata'), findsOneWidget);

      // Tap Refresh Metadata
      await tester.tap(find.text('Refresh Metadata'), warnIfMissed: false);
      await tester.pump();
    },
  );
}
