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
      // In the app the page sits inside MainShell's Scaffold.
      home: Scaffold(body: ShowDetailPage(show: show)),
    ),
  );
}

void main() {
  setUp(() {
    TestWidgetsFlutterBinding.ensureInitialized();
  });

  testWidgets(
    'Show Detail Page lists seasons with season downloads; episodes are optional',
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() => tester.view.resetPhysicalSize());

      final fakeBridge = FakeCoreBridge(mode: FakeMode.data);
      final state = StateProvider(bridge: fakeBridge, startPolling: false);
      await state.refreshAll();

      const testShow = LibraryShow(
        malId: 52991,
        title: 'Sousou no Frieren',
        englishTitle: 'Frieren: Beyond Journey\'s End',
        startYear: 2023,
        totalEpisodes: 28,
        downloadedEpisodes: 28,
        watchedEpisodes: 28,
        score: 9.38,
        status: 'Watching',
        airingStatus: 'Finished',
        genres: ['Adventure', 'Drama', 'Fantasy'],
        synopsis: 'During their decade-long quest to defeat the Demon King...',
        folderPath: '/mnt/anime/Sousou no Frieren (2023)',
        auto: true,
      );

      await tester.pumpWidget(createTestApp(state, testShow));
      await tester.pump();

      // Seasons are the unit: no episode rows until asked for.
      expect(find.text('Sousou no Frieren'), findsWidgets);
      expect(find.text('2 SEASONS'), findsOneWidget);
      expect(find.text('Season 1'), findsOneWidget);
      expect(find.text('Season 2'), findsOneWidget);
      expect(find.text('EP 1'), findsNothing);

      // Season 1 is complete, so only Season 2 offers a download.
      expect(find.byTooltip('Download Season'), findsNWidgets(2));
      expect(find.text('Download Season 2'), findsOneWidget);
      await tester.tap(find.text('Download Season 2'));
      await tester.pump();
      expect(fakeBridge.executed.last, contains('54959'));

      // Picking single episodes is an option inside a season.
      await tester.tap(find.text('Choose episodes').last);
      await tester.pump();
      await tester.pump();
      expect(find.text('EP 1'), findsOneWidget);

      // Verify Back to Library button clears selected show
      expect(find.text('Back to Library'), findsOneWidget);
      await tester.tap(find.text('Back to Library'));
      await tester.pump();
      expect(state.selectedShow, isNull);
    },
  );

  testWidgets(
    'Season card indicates needs_input and button opens Action Inbox',
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() => tester.view.resetPhysicalSize());

      final fakeBridge = FakeCoreBridge(mode: FakeMode.data);
      fakeBridge.customShowDetail = const LibraryShow(
        malId: 18119,
        title: 'Servant x Service',
        englishTitle: 'Servant x Service',
        startYear: 2013,
        totalEpisodes: 13,
        downloadedEpisodes: 0,
        watchedEpisodes: 0,
        score: 7.5,
        status: 'Watching',
        airingStatus: 'Finished',
        genres: ['Comedy', 'Slice of Life'],
        synopsis: 'Yamagami Lucy has begun working at a government office.',
        seasons: [
          SeasonItem(
            malId: 18119,
            title: 'Servant x Service',
            season: 1,
            totalEpisodes: 13,
            downloadedEpisodes: 0,
            activeJobs: 1,
            needsInput: 1,
          ),
        ],
      );

      final state = StateProvider(bridge: fakeBridge, startPolling: false);
      await state.refreshAll();

      final testShow = fakeBridge.customShowDetail!;
      await tester.pumpWidget(createTestApp(state, testShow));
      await tester.pump();

      // Card indicates waiting on user choice
      expect(find.text('Waiting for your choice'), findsOneWidget);
      expect(find.text('WAITING FOR CHOICE'), findsOneWidget);
      expect(find.text('Open Inbox'), findsOneWidget);

      // Tapping "Open Inbox" navigates to Inbox (page index 1)
      await tester.tap(find.text('Open Inbox'));
      await tester.pump();
      expect(state.selectedPageIndex, 1);
      expect(state.selectedShow, isNull);
    },
  );
}
