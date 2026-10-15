import '../../models/activity_item.dart';
import '../../models/library_item.dart';
import 'state_context.dart';

mixin ShowDetailStateMixin on StateContext {
  LibraryShow? _selectedShow;

  @override
  LibraryShow? get selectedShow => _selectedShow;

  @override
  set selectedShow(LibraryShow? value) {
    _selectedShow = value;
  }

  void openShowDetail(LibraryShow show) {
    _selectedShow = show;
    notifyListeners();
    loadShowDetail(show.malId);
  }

  void closeShowDetail() {
    _selectedShow = null;
    notifyListeners();
  }

  @override
  Future<void> loadShowDetail(int malId) async {
    final detail = await bridge.getShowDetail(malId);
    if (detail != null) {
      _selectedShow = detail;
      final idx = shows.indexWhere((s) => s.malId == malId);
      if (idx != -1) {
        shows[idx] = detail;
      }
      notifyListeners();
    }
  }

  Future<void> refreshShowMetadata(int malId, {bool force = false}) async {
    if (_selectedShow != null && _selectedShow!.malId == malId) {
      _selectedShow = _selectedShow!.copyWith(metadataStatus: 'fetching');
      notifyListeners();
    }
    await bridge.fetchShowMetadata(malId, force: force);
    await loadShowDetail(malId);
    await refreshEvents();
  }

  Future<void> addAlias(int malId, String alias) async {
    if (alias.trim().isEmpty) return;
    await bridge.addAlias(malId, alias.trim());
    await loadShowDetail(malId);
  }

  Future<void> removeAlias(int malId, String alias) async {
    await bridge.removeAlias(malId, alias);
    await loadShowDetail(malId);
  }

  Future<void> setEpisodeOffset(int malId, int offset) async {
    await bridge.setEpisodeOffset(malId, offset);
    await loadShowDetail(malId);
  }

  @override
  Future<void> downloadEpisode(int malId, int episodeNumber) async {
    await bridge.downloadEpisode(malId, episodeNumber);
    addActivityEvent(
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Dispatched download for episode $episodeNumber of show #$malId',
        timestamp: DateTime.now(),
        malId: malId,
      ),
    );
    await Future.wait([
      refreshQueue(),
      refreshStatus(),
      refreshEvents(),
      loadShowDetail(malId),
    ]);
  }

  Future<void> downloadSeason(int malId) async {
    await bridge.downloadSeason(malId);
    addActivityEvent(
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Dispatched season download for show #$malId',
        timestamp: DateTime.now(),
        malId: malId,
      ),
    );
    await Future.wait([
      refreshQueue(),
      refreshStatus(),
      refreshEvents(),
      loadShowDetail(malId),
    ]);
  }

  Future<bool> saveShowFolder(int malId, String path) async {
    final res = await bridge.setLocation('folder_path', path, malId: malId);
    addActivityEvent(
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Updated folder override for show #$malId to "$path"',
        timestamp: DateTime.now(),
        malId: malId,
      ),
    );
    final idx = shows.indexWhere((s) => s.malId == malId);
    if (idx != -1) {
      shows[idx] = shows[idx].copyWith(folderPath: path);
    }
    if (_selectedShow?.malId == malId) {
      _selectedShow = _selectedShow!.copyWith(folderPath: path);
    }
    notifyListeners();
    return res != null && (res['is_valid'] == true);
  }
}
