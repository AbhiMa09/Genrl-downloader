import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:google_fonts/google_fonts.dart';
import 'package:genrl_downloader/models/library_item.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/pages/wizard/wizard_series_manifest.dart';

import 'scratch_settings_regression_test.dart' show FixtureBridge;
import 'test_font_helper.dart';

LibraryShow season(int id, {String status = 'watching'}) => LibraryShow(
  malId: id,
  title: 'Fixture franchise',
  startYear: 2026,
  totalEpisodes: 12,
  downloadedEpisodes: 0,
  watchedEpisodes: 0,
  score: 0,
  status: status,
  airingStatus: 'finished',
  genres: const [],
  synopsis: '',
  season: id,
  metadataStatus: id == 2 ? 'pending' : 'partial',
);

class PlanFixture extends StateProvider {
  PlanFixture(String dir)
    : super(bridge: FixtureBridge(dir), startPolling: false);
  List<LibraryShow> fixtureShows = [];
  List<Map<String, dynamic>> fixtureEpisodes = [];
  @override
  List<LibraryShow> get shows => fixtureShows;
  @override
  List<Map<String, dynamic>> get plannedEpisodesList => fixtureEpisodes;
}

void main() {
  late Directory scratch;
  late PlanFixture state;
  setUpAll(() async {
    GoogleFonts.config.allowRuntimeFetching = false;
    await loadAppFonts();
  });
  setUp(() {
    scratch = Directory.systemTemp.createTempSync('genrl-season-fixture-');
    state = PlanFixture(scratch.path);
  });
  tearDown(() {
    state.dispose();
    scratch.deleteSync();
  });
  Future<void> render(WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: WizardSeriesManifest(state: state),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('every watching season stays visible without planned episodes', (
    tester,
  ) async {
    state.fixtureShows = [
      for (var id = 1; id <= 13; id++) season(id),
      season(99, status: 'completed'),
    ];
    await render(tester);
    expect(find.text('Season review • 13 seasons'), findsOneWidget);
    expect(find.byKey(const ValueKey('plan_season_13')), findsOneWidget);
    expect(find.byKey(const ValueKey('plan_season_99')), findsNothing);
    expect(find.textContaining('No episodes planned'), findsNWidgets(13));
    expect(find.textContaining('Metadata unresolved'), findsOneWidget);
  });
  testWidgets(
    'seasons with identical titles stay separate and episodes are exact',
    (tester) async {
      state.fixtureShows = [season(1), season(2)];
      state.fixtureEpisodes = [
        {'mal_id': 1, 'episode': 3},
        {'mal_id': 1, 'episode': 1},
        {'mal_id': 1, 'episode': 3},
        {'mal_id': 2, 'episode': 12.5},
      ];
      await render(tester);
      expect(find.text('Season review • 2 seasons'), findsOneWidget);
      await tester.tap(
        find.descendant(
          of: find.byKey(const ValueKey('plan_season_1')),
          matching: find.byType(ExpansionTile),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('Episode 1'), findsOneWidget);
      expect(find.text('Episode 3'), findsOneWidget);
      expect(find.text('Episode 2'), findsNothing);
      expect(find.textContaining('2 episodes planned'), findsOneWidget);
    },
  );
  testWidgets('empty plan never invents eleven seasons', (tester) async {
    await render(tester);
    expect(find.text('Season review • 0 seasons'), findsOneWidget);
    expect(
      find.text('No watching seasons or planned downloads.'),
      findsOneWidget,
    );
  });
}
