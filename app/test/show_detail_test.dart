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

  testWidgets('Show Detail Page renders show info, folder override, and episodes matrix', (tester) async {
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

    // Verify title, synopsis, and matrix
    expect(find.text('Sousou no Frieren'), findsWidgets);
    expect(find.text('EPISODES MATRIX'), findsOneWidget);
    expect(find.text('EP 1'), findsOneWidget);
    expect(find.text('EP 28'), findsOneWidget);

    // Verify Season Download action in AppBar
    expect(find.byTooltip('Download Season'), findsOneWidget);
    await tester.tap(find.byTooltip('Download Season'));
    await tester.pump();

    // Verify Episode tile click
    await tester.tap(find.text('EP 1'));
    await tester.pump();

    // Verify Back to Library button clears selected show
    expect(find.text('Back to Library'), findsOneWidget);
    await tester.tap(find.text('Back to Library'));
    await tester.pump();
    expect(state.selectedShow, isNull);
  });
}
