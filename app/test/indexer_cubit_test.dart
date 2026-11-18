import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/models/indexer_item.dart';
import 'package:genrl_downloader/state/indexer_cubit.dart';

import 'fake_core_bridge.dart';

class _FakeIndexerBridge extends FakeCoreBridge {
  List<IndexerSummary> indexers = [
    const IndexerSummary(
      id: 'idx_1',
      name: 'Test Tracker',
      baseUrl: 'https://tracker.org',
      enabled: true,
      kinds: ['movie', 'series'],
      hasKey: true,
      lastTestStatus: 'ok',
    ),
  ];

  bool tmdbConfigured = true;

  @override
  Future<ProcessResult> execute(
    List<String> args, {
    Map<String, String>? environment,
  }) async {
    executed.add(args.join(' '));

    if (args.contains('indexers') && args.contains('list')) {
      final jsonList = indexers
          .map(
            (i) => {
              'id': i.id,
              'name': i.name,
              'base_url': i.baseUrl,
              'enabled': i.enabled,
              'kinds': i.kinds,
              'has_key': i.hasKey,
              'last_test_status': i.lastTestStatus,
            },
          )
          .toList();
      return ProcessResult(0, 0, jsonEncode(jsonList), '');
    }

    if (args.contains('indexers') && args.contains('set-enabled')) {
      final id = args[2];
      final enabled = args[3] == 'true';
      indexers = indexers.map((i) {
        if (i.id == id) {
          return IndexerSummary(
            id: i.id,
            name: i.name,
            baseUrl: i.baseUrl,
            enabled: enabled,
            kinds: i.kinds,
            hasKey: i.hasKey,
            lastTestStatus: i.lastTestStatus,
          );
        }
        return i;
      }).toList();
      return ProcessResult(0, 0, '{"ok": true}', '');
    }

    if (args.contains('indexers') && args.contains('remove')) {
      final id = args[2];
      indexers.removeWhere((i) => i.id == id);
      return ProcessResult(0, 0, '{"ok": true}', '');
    }

    if (args.contains('indexers') && args.contains('test')) {
      return ProcessResult(
        0,
        0,
        jsonEncode({
          'ok': true,
          'categories_count': 14,
          'movie_search': true,
          'tv_search': true,
        }),
        '',
      );
    }

    if (args.contains('settings') && args.contains('tmdb-status')) {
      return ProcessResult(
        0,
        0,
        jsonEncode({'configured': tmdbConfigured}),
        '',
      );
    }

    if (args.contains('settings') && args.contains('test-tmdb')) {
      return ProcessResult(0, 0, '{"ok": true}', '');
    }

    return super.execute(args, environment: environment);
  }
}

void main() {
  group('IndexerCubit Unit Tests', () {
    late _FakeIndexerBridge bridge;
    late IndexerCubit cubit;

    setUp(() {
      bridge = _FakeIndexerBridge();
      cubit = IndexerCubit(bridge: bridge);
    });

    tearDown(() {
      cubit.dispose();
    });

    test('loadIndexers loads indexers list and clears loading', () async {
      expect(cubit.state.isLoadingIndexers, isFalse);
      await cubit.loadIndexers();

      expect(cubit.state.indexers.length, equals(1));
      expect(cubit.state.indexers.first.id, equals('idx_1'));
      expect(cubit.state.indexers.first.name, equals('Test Tracker'));
      expect(cubit.state.isLoadingIndexers, isFalse);
    });

    test('setIndexerEnabled toggles state and refreshes', () async {
      await cubit.loadIndexers();
      expect(cubit.state.indexers.first.enabled, isTrue);

      final ok = await cubit.setIndexerEnabled('idx_1', false);
      expect(ok, isTrue);
      expect(cubit.state.indexers.first.enabled, isFalse);
    });

    test('removeIndexer removes item from indexers list', () async {
      await cubit.loadIndexers();
      expect(cubit.state.indexers.length, equals(1));

      final ok = await cubit.removeIndexer('idx_1');
      expect(ok, isTrue);
      expect(cubit.state.indexers, isEmpty);
    });

    test('testIndexer runs probe and stores last result', () async {
      final res = await cubit.testIndexer('idx_1');
      expect(res.ok, isTrue);
      expect(res.movieSearch, isTrue);
      expect(res.tvSearch, isTrue);
      expect(cubit.state.lastIndexerTestResult?.ok, isTrue);
    });

    test('loadTmdbStatus updates isTmdbConfigured', () async {
      await cubit.loadTmdbStatus();
      expect(cubit.state.isTmdbConfigured, isTrue);

      bridge.tmdbConfigured = false;
      await cubit.loadTmdbStatus();
      expect(cubit.state.isTmdbConfigured, isFalse);
    });

    test(
      'testTmdb executes test probe and updates lastTmdbTestSuccess',
      () async {
        final ok = await cubit.testTmdb();
        expect(ok, isTrue);
        expect(cubit.state.lastTmdbTestSuccess, isTrue);
      },
    );
  });
}
