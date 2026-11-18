import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/models/media_item.dart';
import 'package:genrl_downloader/pages/dramas_page.dart';
import 'package:genrl_downloader/pages/movies_page.dart';
import 'package:genrl_downloader/services/core_bridge.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/state/dramas_cubit.dart';
import 'package:genrl_downloader/state/indexer_cubit.dart';
import 'package:genrl_downloader/state/movies_cubit.dart';
import 'package:genrl_downloader/widgets/nav_rail.dart';
import 'package:genrl_downloader/widgets/poster_grid.dart';
import 'package:provider/provider.dart';

import 'fake_core_bridge.dart';
import 'test_font_helper.dart';

class _FakeMediaBridge extends FakeCoreBridge {
  List<MediaListItem> movies = [];
  List<MediaListItem> dramas = [];

  @override
  Future<ProcessResult> execute(
    List<String> args, {
    Map<String, String>? environment,
  }) async {
    executed.add(args.join(' '));

    if (args.contains('media') && args.contains('list')) {
      final kindIdx = args.indexOf('--kind');
      final kind = kindIdx != -1 && kindIdx + 1 < args.length
          ? args[kindIdx + 1]
          : 'movie';
      final list = kind == 'series' ? dramas : movies;
      final jsonList = list
          .map(
            (m) => {
              'id': m.id,
              'kind': m.kind,
              'title': m.title,
              'year': m.year,
              'tmdb_id': m.tmdbId,
              'status': m.status,
              'progress': m.progress != null
                  ? {
                      'wanted': m.progress!.wanted,
                      'queued': m.progress!.queued,
                      'done': m.progress!.done,
                      'unaired': m.progress!.unaired,
                    }
                  : null,
              'poster_path': m.posterPath,
              'overview': m.overview,
            },
          )
          .toList();
      return ProcessResult(0, 0, jsonEncode(jsonList), '');
    }

    if (args.contains('media') && args.contains('remove')) {
      return ProcessResult(0, 0, '{"ok": true}', '');
    }

    if (args.contains('settings') && args.contains('tmdb-status')) {
      return ProcessResult(0, 0, '{"configured": true}', '');
    }

    return super.execute(args, environment: environment);
  }
}

Widget _wrapWithProviders({
  required Widget child,
  required CoreBridge bridge,
  MoviesCubit? moviesCubit,
  DramasCubit? dramasCubit,
  IndexerCubit? indexerCubit,
}) {
  return MultiProvider(
    providers: [
      ChangeNotifierProvider<MoviesCubit>.value(
        value: moviesCubit ?? MoviesCubit(bridge: bridge),
      ),
      ChangeNotifierProvider<DramasCubit>.value(
        value: dramasCubit ?? DramasCubit(bridge: bridge),
      ),
      ChangeNotifierProvider<IndexerCubit>.value(
        value: indexerCubit ?? IndexerCubit(bridge: bridge),
      ),
    ],
    child: MaterialApp(home: Scaffold(body: child)),
  );
}

void main() {
  setUpAll(() async {
    await loadAppFonts();
  });

  group('MoviesPage Widget Tests', () {
    testWidgets('renders empty state when no movies added', (tester) async {
      final bridge = _FakeMediaBridge();
      final moviesCubit = MoviesCubit(bridge: bridge);

      await tester.pumpWidget(
        _wrapWithProviders(
          child: const MoviesPage(),
          bridge: bridge,
          moviesCubit: moviesCubit,
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('No movies yet'), findsOneWidget);
      expect(find.widgetWithText(ElevatedButton, 'Add movie'), findsWidgets);
    });

    testWidgets('renders movies in PosterGrid and opens detail dialog on tap', (
      tester,
    ) async {
      final bridge = _FakeMediaBridge();
      bridge.movies = [
        const MediaListItem(
          id: 42,
          kind: 'movie',
          title: 'Spirited Away',
          year: 2001,
          status: 'in_library',
          overview: 'Chihiro wanders into a world ruled by gods.',
        ),
      ];

      final moviesCubit = MoviesCubit(bridge: bridge);

      await tester.pumpWidget(
        _wrapWithProviders(
          child: const MoviesPage(),
          bridge: bridge,
          moviesCubit: moviesCubit,
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Spirited Away'), findsOneWidget);
      expect(find.text('2001'), findsOneWidget);
      expect(find.byType(PosterGrid), findsOneWidget);

      // Tap the movie card to open detail dialog
      await tester.tap(find.text('Spirited Away'));
      await tester.pumpAndSettle();

      expect(
        find.text('Chihiro wanders into a world ruled by gods.'),
        findsOneWidget,
      );
      expect(find.text('Remove'), findsOneWidget);
      expect(find.text('Close'), findsOneWidget);

      // Close dialog
      await tester.tap(find.text('Close'));
      await tester.pumpAndSettle();
      expect(find.text('Close'), findsNothing);
    });
  });

  group('DramasPage Widget Tests', () {
    testWidgets('renders empty state when no dramas added', (tester) async {
      final bridge = _FakeMediaBridge();
      final dramasCubit = DramasCubit(bridge: bridge);

      await tester.pumpWidget(
        _wrapWithProviders(
          child: const DramasPage(),
          bridge: bridge,
          dramasCubit: dramasCubit,
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('No dramas yet'), findsOneWidget);
      expect(find.widgetWithText(ElevatedButton, 'Add drama'), findsWidgets);
    });

    testWidgets('renders dramas with progress label and opens detail dialog', (
      tester,
    ) async {
      final bridge = _FakeMediaBridge();
      bridge.dramas = [
        const MediaListItem(
          id: 99,
          kind: 'series',
          title: 'Crash Landing on You',
          year: 2019,
          status: 'downloading',
          progress: MediaProgress(done: 8, wanted: 4),
          overview: 'A South Korean heiress accidentally paraglides into North Korea.',
        ),
      ];

      final dramasCubit = DramasCubit(bridge: bridge);

      await tester.pumpWidget(
        _wrapWithProviders(
          child: const DramasPage(),
          bridge: bridge,
          dramasCubit: dramasCubit,
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Crash Landing on You'), findsOneWidget);
      expect(find.text('S1 · 8/12'), findsOneWidget);

      // Tap card to open detail dialog
      await tester.tap(find.text('Crash Landing on You'));
      await tester.pumpAndSettle();

      expect(
        find.text(
          'A South Korean heiress accidentally paraglides into North Korea.',
        ),
        findsOneWidget,
      );
      expect(find.text('Remove'), findsOneWidget);

      await tester.tap(find.text('Close'));
      await tester.pumpAndSettle();
    });
  });

  group('NavRail full order and navigation', () {
    testWidgets(
      'asserts visible top-to-bottom order and index mapping for all 10 items',
      (tester) async {
        final bridge = _FakeMediaBridge();
        final stateProvider = StateProvider(
          bridge: bridge,
          startPolling: false,
        );

        await tester.pumpWidget(
          ChangeNotifierProvider<StateProvider>.value(
            value: stateProvider,
            child: const MaterialApp(home: Scaffold(body: NavRail())),
          ),
        );
        await tester.pump();

        const expectedItems = <(String, int)>[
          ('Dashboard', 0),
          ('Inbox', 1),
          ('Queue', 2),
          ('Library', 3),
          ('Movies', 8),
          ('Dramas', 9),
          ('Search', 4),
          ('Activity', 5),
          ('Rules', 6),
          ('Settings', 7),
        ];

        double previousY = -1.0;
        for (final item in expectedItems) {
          final label = item.$1;
          final expectedIndex = item.$2;

          final finder = find.text(label);
          expect(finder, findsOneWidget, reason: '$label must be present');
          final currentY = tester.getTopLeft(finder).dy;
          expect(
            currentY,
            greaterThan(previousY),
            reason: '$label must appear below the previous navigation item',
          );
          previousY = currentY;

          await tester.tap(finder);
          await tester.pump();
          expect(
            stateProvider.selectedPageIndex,
            equals(expectedIndex),
            reason: 'Tapping $label must select page index $expectedIndex',
          );
        }

        stateProvider.dispose();
      },
    );
  });
}
