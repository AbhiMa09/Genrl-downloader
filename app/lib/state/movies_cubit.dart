import 'package:flutter/foundation.dart';

import '../models/media_item.dart';
import '../services/core_bridge.dart';
import '../services/core_bridge_media.dart';

class MoviesState {
  final List<MediaListItem> movies;
  final bool isLoading;
  final String? error;
  final List<MediaSearchResult> searchResults;
  final bool isSearching;
  final String? searchError;

  const MoviesState({
    this.movies = const [],
    this.isLoading = false,
    this.error,
    this.searchResults = const [],
    this.isSearching = false,
    this.searchError,
  });

  MoviesState copyWith({
    List<MediaListItem>? movies,
    bool? isLoading,
    String? error,
    bool clearError = false,
    List<MediaSearchResult>? searchResults,
    bool? isSearching,
    String? searchError,
    bool clearSearchError = false,
  }) {
    return MoviesState(
      movies: movies ?? this.movies,
      isLoading: isLoading ?? this.isLoading,
      error: clearError ? null : (error ?? this.error),
      searchResults: searchResults ?? this.searchResults,
      isSearching: isSearching ?? this.isSearching,
      searchError: clearSearchError ? null : (searchError ?? this.searchError),
    );
  }
}

class MoviesCubit extends ChangeNotifier {
  final CoreBridge bridge;
  MoviesState _state = const MoviesState();

  MoviesCubit({required this.bridge});

  MoviesState get state => _state;

  Future<void> loadMovies() async {
    _state = _state.copyWith(isLoading: true, clearError: true);
    notifyListeners();

    try {
      final items = await bridge.listMedia(kind: 'movie');
      _state = _state.copyWith(isLoading: false, movies: items);
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
        kind: 'movie',
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

  Future<bool> addMovie(int tmdbId) async {
    try {
      final outcome = await bridge.addMedia(tmdbId, kind: 'movie');
      if (outcome.ok) {
        await loadMovies();
        return true;
      }
      _state = _state.copyWith(
        error: outcome.error?.message ?? 'Failed to add movie',
      );
      notifyListeners();
    } catch (e) {
      _state = _state.copyWith(error: e.toString());
      notifyListeners();
    }
    return false;
  }

  Future<bool> removeMovie(int mediaId) async {
    try {
      final ok = await bridge.removeMedia(mediaId);
      if (ok) {
        await loadMovies();
        return true;
      }
    } catch (e) {
      _state = _state.copyWith(error: e.toString());
      notifyListeners();
    }
    return false;
  }
}
