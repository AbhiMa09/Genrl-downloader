import 'package:flutter/foundation.dart';

import '../models/indexer_item.dart';
import '../services/core_bridge.dart';
import '../services/core_bridge_indexers.dart';

/// State representation for Indexers and TMDB configuration.
/// Strictly enforces secret hygiene: NEVER stores raw API keys or tokens.
class IndexerState {
  final List<IndexerSummary> indexers;
  final bool isLoadingIndexers;
  final String? indexersError;
  final bool isTestingIndexer;
  final String? testingIndexerId;
  final IndexerTestResult? lastIndexerTestResult;
  final bool isTmdbConfigured;
  final bool isTestingTmdb;
  final bool? lastTmdbTestSuccess;
  final String? tmdbError;

  const IndexerState({
    this.indexers = const [],
    this.isLoadingIndexers = false,
    this.indexersError,
    this.isTestingIndexer = false,
    this.testingIndexerId,
    this.lastIndexerTestResult,
    this.isTmdbConfigured = false,
    this.isTestingTmdb = false,
    this.lastTmdbTestSuccess,
    this.tmdbError,
  });

  IndexerState copyWith({
    List<IndexerSummary>? indexers,
    bool? isLoadingIndexers,
    String? indexersError,
    bool clearIndexersError = false,
    bool? isTestingIndexer,
    String? testingIndexerId,
    bool clearTestingIndexerId = false,
    IndexerTestResult? lastIndexerTestResult,
    bool clearLastIndexerTestResult = false,
    bool? isTmdbConfigured,
    bool? isTestingTmdb,
    bool? lastTmdbTestSuccess,
    bool clearLastTmdbTestSuccess = false,
    String? tmdbError,
    bool clearTmdbError = false,
  }) {
    return IndexerState(
      indexers: indexers ?? this.indexers,
      isLoadingIndexers: isLoadingIndexers ?? this.isLoadingIndexers,
      indexersError: clearIndexersError
          ? null
          : (indexersError ?? this.indexersError),
      isTestingIndexer: isTestingIndexer ?? this.isTestingIndexer,
      testingIndexerId: clearTestingIndexerId
          ? null
          : (testingIndexerId ?? this.testingIndexerId),
      lastIndexerTestResult: clearLastIndexerTestResult
          ? null
          : (lastIndexerTestResult ?? this.lastIndexerTestResult),
      isTmdbConfigured: isTmdbConfigured ?? this.isTmdbConfigured,
      isTestingTmdb: isTestingTmdb ?? this.isTestingTmdb,
      lastTmdbTestSuccess: clearLastTmdbTestSuccess
          ? null
          : (lastTmdbTestSuccess ?? this.lastTmdbTestSuccess),
      tmdbError: clearTmdbError ? null : (tmdbError ?? this.tmdbError),
    );
  }
}

class IndexerCubit extends ChangeNotifier {
  final CoreBridge bridge;
  IndexerState _state = const IndexerState();

  IndexerCubit({required this.bridge});

  IndexerState get state => _state;

  Future<void> loadIndexers() async {
    _state = _state.copyWith(isLoadingIndexers: true, clearIndexersError: true);
    notifyListeners();

    try {
      final items = await bridge.listIndexers();
      _state = _state.copyWith(isLoadingIndexers: false, indexers: items);
    } catch (e) {
      _state = _state.copyWith(
        isLoadingIndexers: false,
        indexersError: e.toString(),
      );
    }
    notifyListeners();
  }

  Future<bool> addIndexer({
    required String id,
    required String name,
    required String url,
    required String apiKey,
    List<String>? kinds,
    bool test = false,
  }) async {
    try {
      final added = await bridge.addIndexer(
        id: id,
        name: name,
        url: url,
        kinds: kinds ?? const ['movie', 'series'],
        apiKey: apiKey,
        test: test,
      );
      if (added != null) {
        await loadIndexers();
        return true;
      }
    } catch (e) {
      _state = _state.copyWith(indexersError: e.toString());
      notifyListeners();
    }
    return false;
  }

  Future<bool> removeIndexer(String id) async {
    try {
      final ok = await bridge.removeIndexer(id);
      if (ok) {
        await loadIndexers();
        return true;
      }
    } catch (e) {
      _state = _state.copyWith(indexersError: e.toString());
      notifyListeners();
    }
    return false;
  }

  Future<IndexerTestResult> testIndexer(String id) async {
    _state = _state.copyWith(
      isTestingIndexer: true,
      testingIndexerId: id,
      clearLastIndexerTestResult: true,
    );
    notifyListeners();

    try {
      final res =
          await bridge.testIndexer(id) ??
          const IndexerTestResult(ok: false, error: 'No response from test');
      _state = _state.copyWith(
        isTestingIndexer: false,
        clearTestingIndexerId: true,
        lastIndexerTestResult: res,
      );
      await loadIndexers(); // Refresh test status from DB
      notifyListeners();
      return res;
    } catch (e) {
      final fail = IndexerTestResult(ok: false, error: e.toString());
      _state = _state.copyWith(
        isTestingIndexer: false,
        clearTestingIndexerId: true,
        lastIndexerTestResult: fail,
      );
      notifyListeners();
      return fail;
    }
  }

  Future<bool> setIndexerEnabled(String id, bool enabled) async {
    try {
      final ok = await bridge.setIndexerEnabled(id, enabled);
      if (ok) {
        await loadIndexers();
        return true;
      }
    } catch (e) {
      _state = _state.copyWith(indexersError: e.toString());
      notifyListeners();
    }
    return false;
  }

  Future<void> loadTmdbStatus() async {
    try {
      final configured = await bridge.getTmdbStatus();
      _state = _state.copyWith(isTmdbConfigured: configured);
    } catch (_) {}
    notifyListeners();
  }

  Future<bool> setTmdbKey(String apiKey) async {
    try {
      final err = await bridge.setTmdbKey(apiKey);
      if (err == null) {
        await loadTmdbStatus();
        return true;
      }
      _state = _state.copyWith(tmdbError: err.message);
      notifyListeners();
    } catch (e) {
      _state = _state.copyWith(tmdbError: e.toString());
      notifyListeners();
    }
    return false;
  }

  Future<bool> testTmdb() async {
    _state = _state.copyWith(
      isTestingTmdb: true,
      clearLastTmdbTestSuccess: true,
      clearTmdbError: true,
    );
    notifyListeners();

    try {
      final res = await bridge.testTmdb();
      final ok = res.ok;
      _state = _state.copyWith(
        isTestingTmdb: false,
        lastTmdbTestSuccess: ok,
        tmdbError: ok ? null : res.error?.message,
      );
      notifyListeners();
      return ok;
    } catch (e) {
      _state = _state.copyWith(
        isTestingTmdb: false,
        lastTmdbTestSuccess: false,
        tmdbError: e.toString(),
      );
      notifyListeners();
      return false;
    }
  }
}
