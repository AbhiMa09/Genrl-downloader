import 'package:flutter/foundation.dart';

import '../models/media_item.dart';
import '../services/core_bridge.dart';
import '../services/core_bridge_media.dart';

class DramasState {
  final List<MediaListItem> dramas;
  final bool isLoading;
  final String? error;
  final List<MediaSearchResult> searchResults;
  final bool isSearching;
  final String? searchError;
  final MediaListItem? selectedDrama;
  final List<MediaEpisodeItem> episodes;
  final bool isLoadingEpisodes;

  const DramasState({
    this.dramas = const [],
    this.isLoading = false,
    this.error,
    this.searchResults = const [],
    this.isSearching = false,
    this.searchError,
    this.selectedDrama,
    this.episodes = const [],
    this.isLoadingEpisodes = false,
  });

  DramasState copyWith({
    List<MediaListItem>? dramas,
    bool? isLoading,
    String? error,
    bool clearError = false,
    List<MediaSearchResult>? searchResults,
    bool? isSearching,
    String? searchError,
    bool clearSearchError = false,
    MediaListItem? selectedDrama,
    bool clearSelectedDrama = false,
    List<MediaEpisodeItem>? episodes,
    bool? isLoadingEpisodes,
  }) {
    return DramasState(
      dramas: dramas ?? this.dramas,
      isLoading: isLoading ?? this.isLoading,
      error: clearError ? null : (error ?? this.error),
      searchResults: searchResults ?? this.searchResults,
      isSearching: isSearching ?? this.isSearching,
      searchError: clearSearchError ? null : (searchError ?? this.searchError),
      selectedDrama: clearSelectedDrama
          ? null
          : (selectedDrama ?? this.selectedDrama),
      episodes: episodes ?? this.episodes,
      isLoadingEpisodes: isLoadingEpisodes ?? this.isLoadingEpisodes,
    );
  }
}

class DramasCubit extends ChangeNotifier {
  final CoreBridge bridge;
  DramasState _state = const DramasState();

  DramasCubit({required this.bridge});

  DramasState get state => _state;

  Future<void> loadDramas() async {
    _state = _state.copyWith(isLoading: true, clearError: true);
    notifyListeners();

    try {
      final items = await bridge.listMedia(kind: 'series');
      _state = _state.copyWith(isLoading: false, dramas: items);
    } catch (e) {
      _state = _state.copyWith(isLoading: false, error: e.toString());
    }
    notifyListeners();
  }

  Future<void> search(String query, {int? year}) async {
    final trimmed = query.trim();
    if (trimmed.isEmpty) {
      _state = _state.copyWith(
        searchResults: const [],
        isSearching: false,
        clearSearchError: true,
      );
      notifyListeners();
      return;
    }

    _state = _state.copyWith(isSearching: true, clearSearchError: true);
    notifyListeners();

    try {
      final outcome = await bridge.searchMedia(
        trimmed,
        kind: 'series',
        year: year,
      );
      if (outcome.ok) {
        _state = _state.copyWith(
          isSearching: false,
          searchResults: outcome.items,
        );
      } else {
        _state = _state.copyWith(
          isSearching: false,
          searchError: outcome.error?.message ?? 'Search failed',
        );
      }
    } catch (e) {
      _state = _state.copyWith(isSearching: false, searchError: e.toString());
    }
    notifyListeners();
  }

  void clearSearch() {
    _state = _state.copyWith(
      searchResults: const [],
      isSearching: false,
      clearSearchError: true,
    );
    notifyListeners();
  }

  Future<bool> addDrama(int tmdbId) async {
    try {
      final outcome = await bridge.addMedia(tmdbId, kind: 'series');
      if (outcome.ok) {
        await loadDramas();
        return true;
      }
      _state = _state.copyWith(
        error: outcome.error?.message ?? 'Failed to add drama',
      );
      notifyListeners();
    } catch (e) {
      _state = _state.copyWith(error: e.toString());
      notifyListeners();
    }
    return false;
  }

  Future<bool> removeDrama(int mediaId) async {
    try {
      final ok = await bridge.removeMedia(mediaId);
      if (ok) {
        await loadDramas();
        return true;
      }
    } catch (e) {
      _state = _state.copyWith(error: e.toString());
      notifyListeners();
    }
    return false;
  }

  void openDramaDetail(MediaListItem item) {
    _state = _state.copyWith(selectedDrama: item, episodes: const []);
    notifyListeners();
    loadEpisodes(item.id);
  }

  void closeDramaDetail() {
    _state = _state.copyWith(
      clearSelectedDrama: true,
      episodes: const [],
      isLoadingEpisodes: false,
    );
    notifyListeners();
  }

  Future<void> loadEpisodes(int mediaId, {int? season}) async {
    _state = _state.copyWith(isLoadingEpisodes: true);
    notifyListeners();
    try {
      final eps = await bridge.mediaEpisodes(mediaId, season: season);
      _state = _state.copyWith(isLoadingEpisodes: false, episodes: eps);
    } catch (_) {
      _state = _state.copyWith(isLoadingEpisodes: false);
    }
    notifyListeners();
  }

  Future<bool> downloadSeasonPack(int mediaId, {int season = 1}) async {
    try {
      final res = await bridge.mediaDownload(mediaId, season: season);
      await loadEpisodes(mediaId, season: season);
      return res != null && res['ok'] == true;
    } catch (_) {
      return false;
    }
  }

  Future<bool> downloadEpisode(int mediaId, int season, int episode) async {
    try {
      final res = await bridge.mediaDownload(
        mediaId,
        season: season,
        episode: episode,
      );
      await loadEpisodes(mediaId, season: season);
      return res != null && res['ok'] == true;
    } catch (_) {
      return false;
    }
  }
}
