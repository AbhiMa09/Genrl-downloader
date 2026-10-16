import '../../models/activity_item.dart';
import '../../models/library_item.dart';
import 'state_context.dart';

mixin LibraryStateMixin on StateContext {
  List<LibraryShow> _shows = [];
  bool _isLoadingShows = false;
  String? _showsError;

  @override
  List<LibraryShow> get shows => _shows;
  List<LibraryShow> get trackedShows => _shows;
  bool get isLoadingShows => _isLoadingShows;
  String? get showsError => _showsError;

  void updateShowInList(LibraryShow updated) {
    final idx = _shows.indexWhere((s) => s.malId == updated.malId);
    if (idx != -1) {
      _shows[idx] = updated;
    }
  }

  @override
  Future<void> refreshShows({String? status}) async {
    _isLoadingShows = true;
    _showsError = null;
    try {
      final items = await bridge.getShows(status: status);
      _shows = List<LibraryShow>.from(items);
      if (selectedShow != null) {
        final updated = items
            .where((s) => s.malId == selectedShow!.malId)
            .firstOrNull;
        if (updated != null) {
          selectedShow = updated;
        }
      }
    } catch (e) {
      _showsError = e.toString();
    } finally {
      _isLoadingShows = false;
      notifyListeners();
    }
  }

  Future<void> toggleShowAuto(int malId) async {
    final idx = _shows.indexWhere((s) => s.malId == malId);
    if (idx != -1) {
      final current = _shows[idx];
      final next = !current.auto;
      _shows[idx] = current.copyWith(auto: next);
      notifyListeners();
      await bridge.setShowAuto(malId, next);
      addActivityEvent(
        ActivityItem(
          id: DateTime.now().millisecondsSinceEpoch,
          level: ActivityLevel.info,
          message: 'Show ${current.title} auto-download set to $next',
          timestamp: DateTime.now(),
          malId: malId,
        ),
      );
    }
  }

  Future<void> updateShowDetails({
    required int malId,
    double? rating,
    String? status,
    int? watchedEpisodes,
    int? downloadedEpisodes,
    bool? auto,
  }) async {
    final idx = _shows.indexWhere((s) => s.malId == malId);
    if (idx != -1) {
      final old = _shows[idx];
      final updated = old.copyWith(
        score: rating ?? old.score,
        status: status ?? old.status,
        watchedEpisodes: watchedEpisodes ?? old.watchedEpisodes,
        downloadedEpisodes: downloadedEpisodes ?? old.downloadedEpisodes,
        auto: auto ?? old.auto,
      );
      _shows[idx] = updated;
      if (selectedShow?.malId == malId) {
        selectedShow = updated;
      }
    }
    addActivityEvent(
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message:
            'Updated show #$malId: ${[if (rating != null) "Rating: ★${rating.toStringAsFixed(1)}", if (status != null) "Status: $status", if (watchedEpisodes != null) "Watched: $watchedEpisodes eps"].join(", ")}',
        timestamp: DateTime.now(),
        malId: malId,
      ),
    );
    notifyListeners();
  }

  Future<bool> grabNextAvailable() async {
    final watchingShows = _shows.where((s) => s.status == 'Watching').toList();
    for (final show in watchingShows) {
      if (show.downloadedEpisodes < show.totalEpisodes) {
        final nextEp = show.downloadedEpisodes + 1;
        await downloadEpisode(show.malId, nextEp);
        return true;
      }
    }
    selectPage(4);
    return false;
  }
}
