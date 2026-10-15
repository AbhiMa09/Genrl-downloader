import 'dart:async';
import 'package:flutter/material.dart';
import '../models/activity_item.dart';
import '../models/inbox_item.dart';
import '../models/library_item.dart';
import '../models/queue_item.dart';
import '../models/rule_item.dart';
import '../models/system_status.dart';
import 'core_bridge.dart';

class StateProvider extends ChangeNotifier {
  final CoreBridge bridge;

  int _selectedPageIndex = 0;
  String _searchFilter = '';
  SystemStatus _status = const SystemStatus(daemonConnected: false);
  List<QueueItem> _queue = [];
  List<InboxItem> _inbox = [];
  List<LibraryShow> _shows = [];
  List<ActivityItem> _events = [];
  List<RuleItem> _rules = [];
  LibraryShow? _selectedShow;
  bool _isSyncing = false;
  bool _isScanning = false;

  // Real Core connection & loading states
  bool _isCoreConnected = true;
  String? _coreError;
  bool _isLoadingStatus = false;
  bool _isLoadingQueue = false;
  bool _isLoadingShows = false;
  bool _isLoadingInbox = false;
  bool _isLoadingEvents = false;
  bool _isLoadingRules = false;

  String? _queueError;
  String? _showsError;
  String? _inboxError;
  String? _eventsError;
  String? _rulesError;

  // Background timer for live refresh & event stream simulation
  Timer? _pollingTimer;

  // Locations state
  Map<String, dynamic>? _locationsReport;
  Map<String, dynamic>? _downloadCheck;
  Map<String, dynamic>? _libraryCheck;
  bool _sameFilesystem = true;
  String _filesystemNote = '';
  bool _isMigrating = false;
  String? _migrationResult;

  StateProvider({CoreBridge? bridge, bool startPolling = true})
      : bridge = bridge ?? CoreBridge() {
    refreshAll();
    if (startPolling) {
      _startPolling();
    }
  }

  @override
  void dispose() {
    _pollingTimer?.cancel();
    super.dispose();
  }

  void _startPolling() {
    _pollingTimer?.cancel();
    _pollingTimer = Timer.periodic(const Duration(seconds: 3), (_) {
      // Live background update for Dashboard & Queue
      refreshLiveTelemetry();
    });
  }

  // Getters
  int get selectedPageIndex => _selectedPageIndex;
  String get searchFilter => _searchFilter;
  SystemStatus get status => _status;
  List<QueueItem> get queue => _queue;
  List<InboxItem> get inbox => _inbox;
  List<LibraryShow> get shows => _shows;
  List<ActivityItem> get events => _events;
  List<RuleItem> get rules => _rules;
  LibraryShow? get selectedShow => _selectedShow;
  bool get isSyncing => _isSyncing;
  bool get isScanning => _isScanning;

  bool get isCoreConnected => _isCoreConnected;
  String? get coreError => _coreError;
  bool get isLoadingStatus => _isLoadingStatus;
  bool get isLoadingQueue => _isLoadingQueue;
  bool get isLoadingShows => _isLoadingShows;
  bool get isLoadingInbox => _isLoadingInbox;
  bool get isLoadingEvents => _isLoadingEvents;
  bool get isLoadingRules => _isLoadingRules;

  List<LibraryShow> get trackedShows => _shows;
  List<QueueItem> get activeDownloads => _queue.where((q) => q.state == QueueState.downloading).toList();
  bool get isMalAuthenticated => isMalConnected;

  Future<void> syncNow() => forceSyncMal();
  Future<void> loginMal() => connectMal();
  Future<void> updateLocations({String? downloadDir, String? libraryDir}) async {
    if (downloadDir != null && downloadDir.isNotEmpty) {
      await saveLocation('download_dir', downloadDir);
    }
    if (libraryDir != null && libraryDir.isNotEmpty) {
      await saveLocation('library_dir', libraryDir);
    }
  }

  RuleItem? get globalRule => _rules.where((r) => r.isGlobal).firstOrNull;

  String? get queueError => _queueError;
  String? get showsError => _showsError;
  String? get inboxError => _inboxError;
  String? get eventsError => _eventsError;
  String? get rulesError => _rulesError;

  int get inboxCount => _inbox.length;
  int get activeQueueCount =>
      _queue.where((i) => i.state == QueueState.downloading).length;

  Map<String, dynamic>? get locationsReport => _locationsReport;
  Map<String, dynamic>? get downloadCheck => _downloadCheck;
  Map<String, dynamic>? get libraryCheck => _libraryCheck;
  bool get sameFilesystem => _sameFilesystem;
  String get filesystemNote => _filesystemNote;
  bool get isMigrating => _isMigrating;
  String? get migrationResult => _migrationResult;

  // Navigation
  void selectPage(int index) {
    _selectedPageIndex = index;
    _selectedShow = null;
    notifyListeners();
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

  void setSearchFilter(String query) {
    _searchFilter = query;
    notifyListeners();
  }

  // Core queries and store refresh
  Future<void> refreshAll() async {
    await Future.wait([
      refreshStatus(),
      refreshQueue(),
      refreshShows(),
      refreshInbox(),
      refreshEvents(),
      refreshRules(),
      refreshMalStatus(),
      refreshLocations(),
    ]);
  }

  Future<void> refreshLiveTelemetry() async {
    try {
      final results = await Future.wait([
        bridge.getSystemStatus(),
        bridge.getQueue(),
        bridge.getEvents(),
      ]);
      final statusRes = results[0] as SystemStatus?;
      final queueRes = results[1] as List<QueueItem>?;
      final eventsRes = results[2] as List<ActivityItem>?;

      if (statusRes != null) {
        _status = statusRes;
        _isCoreConnected = true;
        _coreError = null;
      } else {
        _isCoreConnected = false;
        _coreError = 'Core daemon unreachable (Process exit non-zero or genrl not found)';
      }

      if (queueRes != null) {
        _queue = queueRes;
        _queueError = null;
      }

      if (eventsRes != null) {
        _events = eventsRes;
        _eventsError = null;
        if (_selectedShow != null && _selectedShow!.metadataStatus != 'ready') {
          final hasMetadataUpdate = eventsRes.any((e) =>
              (e.malId == _selectedShow!.malId ||
                  e.message.contains('#${_selectedShow!.malId}') ||
                  e.message.toLowerCase().contains(_selectedShow!.title.toLowerCase())) &&
              (e.message.toLowerCase().contains('metadata') || e.message.toLowerCase().contains('synopsis')));
          if (hasMetadataUpdate) {
            await loadShowDetail(_selectedShow!.malId);
          }
        }
      }
    } catch (e) {
      _isCoreConnected = false;
      _coreError = e.toString();
    } finally {
      notifyListeners();
    }
  }

  Future<void> refreshStatus() async {
    _isLoadingStatus = true;
    _coreError = null;
    try {
      final res = await bridge.getSystemStatus();
      if (res != null) {
        _status = res;
        _isCoreConnected = true;
        _coreError = null;
      } else {
        _isCoreConnected = false;
        _coreError = 'Core daemon unreachable (Process exit non-zero or genrl not found)';
      }
    } catch (e) {
      _isCoreConnected = false;
      _coreError = e.toString();
    } finally {
      _isLoadingStatus = false;
      notifyListeners();
    }
  }

  Future<void> refreshQueue() async {
    _isLoadingQueue = true;
    _queueError = null;
    try {
      final items = await bridge.getQueue();
      _queue = items;
    } catch (e) {
      _queueError = e.toString();
    } finally {
      _isLoadingQueue = false;
      notifyListeners();
    }
  }

  Future<void> refreshShows({String? status}) async {
    _isLoadingShows = true;
    _showsError = null;
    try {
      final items = await bridge.getShows(status: status);
      _shows = List<LibraryShow>.from(items);
      if (_selectedShow != null) {
        final updated = items.where((s) => s.malId == _selectedShow!.malId).firstOrNull;
        if (updated != null) {
          _selectedShow = updated;
        }
      }
    } catch (e) {
      _showsError = e.toString();
    } finally {
      _isLoadingShows = false;
      notifyListeners();
    }
  }

  Future<void> refreshInbox() async {
    _isLoadingInbox = true;
    _inboxError = null;
    try {
      final items = await bridge.getInbox();
      _inbox = items;
    } catch (e) {
      _inboxError = e.toString();
    } finally {
      _isLoadingInbox = false;
      notifyListeners();
    }
  }

  Future<void> answerInboxQuestion(int id, String choiceKey, {bool always = false}) async {
    try {
      await bridge.answerQuestion(id, choiceKey, always: always);
    } catch (_) {}
    _inbox.removeWhere((i) => i.id == id);
    notifyListeners();
  }

  Future<void> refreshEvents() async {
    _isLoadingEvents = true;
    _eventsError = null;
    try {
      final items = await bridge.getEvents();
      _events = items;
      if (_selectedShow != null && _selectedShow!.metadataStatus != 'ready') {
        final hasMetadataUpdate = items.any((e) =>
            (e.malId == _selectedShow!.malId ||
                e.message.contains('#${_selectedShow!.malId}') ||
                e.message.toLowerCase().contains(_selectedShow!.title.toLowerCase())) &&
            (e.message.toLowerCase().contains('metadata') || e.message.toLowerCase().contains('synopsis')));
        if (hasMetadataUpdate) {
          await loadShowDetail(_selectedShow!.malId);
        }
      }
    } catch (e) {
      _eventsError = e.toString();
    } finally {
      _isLoadingEvents = false;
      notifyListeners();
    }
  }

  Future<void> loadShowDetail(int malId) async {
    final detail = await bridge.getShowDetail(malId);
    if (detail != null) {
      _selectedShow = detail;
      final idx = _shows.indexWhere((s) => s.malId == malId);
      if (idx != -1) {
        _shows[idx] = detail;
      }
      notifyListeners();
    }
  }

  /// Trigger metadata refresh or retry for a show
  Future<void> refreshShowMetadata(int malId, {bool force = false}) async {
    if (_selectedShow != null && _selectedShow!.malId == malId) {
      _selectedShow = _selectedShow!.copyWith(
        metadataStatus: 'fetching',
      );
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

  // Actions
  Future<void> toggleAutomation() async {
    final nextState = !_status.automationOn;
    if (nextState) {
      await bridge.resumeQueue();
    } else {
      await bridge.pauseQueue();
    }
    await refreshStatus();
  }

  Future<void> forceSyncMal() async {
    _isSyncing = true;
    notifyListeners();
    await bridge.syncMal();
    await Future.wait([
      refreshMalStatus(),
      refreshShows(),
      refreshEvents(),
      refreshStatus(),
    ]);
    _isSyncing = false;
    notifyListeners();
  }

  Future<void> manualDiskScan() async {
    _isScanning = true;
    notifyListeners();
    await bridge.rescanLibrary();
    await Future.wait([
      refreshShows(),
      refreshLocations(),
      refreshEvents(),
    ]);
    _isScanning = false;
    notifyListeners();
  }

  Future<void> answerQuestion(int questionId, String answerKey, {bool always = false}) async {
    await bridge.answerQuestion(questionId, answerKey, always: always);
    await Future.wait([
      refreshInbox(),
      refreshEvents(),
      refreshQueue(),
    ]);
  }

  void removeQueueItem(int id) {
    _queue.removeWhere((item) => item.id == id);
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

  // MAL Status State
  String _malStatus = 'not_connected'; // 'connected', 'public_profile', 'token_expired', 'not_connected'
  String? _malUsername;
  String? _malSource;
  String? _malError;
  String? _malClientId;
  int _watchingCount = 0;
  int _completedCount = 0;
  int _onHoldCount = 0;
  int _droppedCount = 0;
  int _planToWatchCount = 0;

  String get malStatus => _malStatus;
  String? get malUsername => _malUsername;
  String? get malSource => _malSource;
  String? get malError => _malError;
  String? get malClientId => _malClientId;
  int get watchingCount => _watchingCount;
  int get completedCount => _completedCount;
  int get onHoldCount => _onHoldCount;
  int get droppedCount => _droppedCount;
  int get planToWatchCount => _planToWatchCount;

  bool get isMalConnected => _malStatus == 'connected';
  bool get isPublicProfile => _malStatus == 'public_profile';

  Future<void> refreshMalStatus() async {
    try {
      final statusMap = await bridge.getMalStatus();
      if (statusMap != null) {
        _malStatus = statusMap['status'] as String? ?? 'not_connected';
        _malUsername = statusMap['username'] as String?;
        _malSource = statusMap['source'] as String?;
        _malClientId = statusMap['client_id'] as String?;
        final counts = statusMap['counts'] as Map<String, dynamic>? ?? {};
        _watchingCount = (counts['watching'] as num?)?.toInt() ?? 0;
        _completedCount = (counts['completed'] as num?)?.toInt() ?? 0;
        _onHoldCount = (counts['on_hold'] as num?)?.toInt() ?? 0;
        _droppedCount = (counts['dropped'] as num?)?.toInt() ?? 0;
        _planToWatchCount = (counts['plan_to_watch'] as num?)?.toInt() ?? 0;
        notifyListeners();
      }
    } catch (e) {
      _malError = e.toString();
      notifyListeners();
    }
  }

  Future<void> connectMal({
    String? clientId,
    String? clientSecret,
    int? port,
    String? redirectUri,
  }) async {
    _malError = null;
    notifyListeners();
    final res = await bridge.malConnectStart(
      clientId: clientId,
      clientSecret: clientSecret,
      port: port,
      redirectUri: redirectUri,
    );
    if (res.exitCode != 0) {
      _malError = res.stderr.toString().isNotEmpty ? res.stderr.toString() : res.stdout.toString();
    } else {
      if (clientId != null && clientId.isNotEmpty) {
        _malClientId = clientId;
      }
      await refreshMalStatus();
    }
    notifyListeners();
  }

  Future<void> disconnectMal() async {
    await bridge.malDisconnect();
    _malStatus = 'not_connected';
    _malUsername = null;
    _malSource = null;
    _events.insert(
      0,
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'MAL credentials disconnected and session cleared',
        timestamp: DateTime.now(),
      ),
    );
    notifyListeners();
  }

  Future<void> setPublicProfile(String username) async {
    final ok = await bridge.malSetPublicProfile(username);
    if (ok) {
      _malStatus = 'public_profile';
      _malUsername = username;
      _malSource = 'jikan';
      _events.insert(
        0,
        ActivityItem(
          id: DateTime.now().millisecondsSinceEpoch,
          level: ActivityLevel.info,
          message: 'Public MAL profile set to "$username"',
          timestamp: DateTime.now(),
        ),
      );
    }
    notifyListeners();
  }

  // Show Auto-Download Toggle
  Future<void> toggleShowAuto(int malId) async {
    final idx = _shows.indexWhere((s) => s.malId == malId);
    if (idx != -1) {
      final current = _shows[idx];
      final next = !current.auto;
      _shows[idx] = current.copyWith(auto: next);
      notifyListeners();
      await bridge.setShowAuto(malId, next);
      _events.insert(
        0,
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

  Future<void> downloadEpisode(int malId, int episodeNumber) async {
    await bridge.downloadEpisode(malId, episodeNumber);
    _events.insert(
      0,
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
    _events.insert(
      0,
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
      if (_selectedShow?.malId == malId) {
        _selectedShow = updated;
      }
    }
    _events.insert(
      0,
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Updated show #$malId: ${[
          if (rating != null) "Rating: ★${rating.toStringAsFixed(1)}",
          if (status != null) "Status: $status",
          if (watchedEpisodes != null) "Watched: $watchedEpisodes eps",
        ].join(", ")}',
        timestamp: DateTime.now(),
        malId: malId,
      ),
    );
    notifyListeners();
  }

  // Rules State Management
  Future<void> refreshRules() async {
    _isLoadingRules = true;
    _rulesError = null;
    notifyListeners();
    try {
      final globalData = await bridge.getRules();
      if (globalData != null) {
        final globalRule = RuleItem.fromJson(globalData, showTitle: 'Global Defaults');
        final overrides = _rules.where((r) => !r.isGlobal).toList();
        _rules = [globalRule, ...overrides];
      }
    } catch (e) {
      _rulesError = e.toString();
    } finally {
      _isLoadingRules = false;
      notifyListeners();
    }
  }

  Future<void> updateRule(String key, dynamic value, {int? malId}) async {
    await bridge.updateRule(key, value, malId: malId);
    await refreshRules();
    _events.insert(
      0,
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Updated rule "$key" for ${malId != null ? "show #$malId" : "global defaults"}',
        timestamp: DateTime.now(),
        malId: malId,
      ),
    );
    notifyListeners();
  }

  Future<void> resetRule({String? key, int? malId}) async {
    await bridge.resetRule(key: key, malId: malId);
    await refreshRules();
    _events.insert(
      0,
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Reset rule(s) for ${malId != null ? "show #$malId" : "global defaults"}',
        timestamp: DateTime.now(),
        malId: malId,
      ),
    );
    notifyListeners();
  }

  // Locations Management
  Future<void> refreshLocations() async {
    try {
      final report = await bridge.getLocations();
      if (report != null) {
        _locationsReport = report;
        _downloadCheck = report['download_dir'] as Map<String, dynamic>?;
        _libraryCheck = report['library_dir'] as Map<String, dynamic>?;
        _sameFilesystem = report['same_filesystem'] as bool? ?? true;
        _filesystemNote = report['note'] as String? ?? '';
        notifyListeners();
      }
    } catch (_) {}
  }

  Future<Map<String, dynamic>?> checkLocationLive(String path) async {
    return await bridge.checkLocation(path);
  }

  Future<bool> saveLocation(String key, String path) async {
    final res = await bridge.setLocation(key, path);
    await refreshLocations();
    _events.insert(
      0,
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Updated location "$key" to "$path"',
        timestamp: DateTime.now(),
      ),
    );
    notifyListeners();
    return res != null && (res['is_valid'] == true);
  }

  Future<bool> saveShowFolder(int malId, String path) async {
    final res = await bridge.setLocation('folder_path', path, malId: malId);
    _events.insert(
      0,
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Updated folder override for show #$malId to "$path"',
        timestamp: DateTime.now(),
        malId: malId,
      ),
    );
    final idx = _shows.indexWhere((s) => s.malId == malId);
    if (idx != -1) {
      _shows[idx] = _shows[idx].copyWith(folderPath: path);
    }
    if (_selectedShow?.malId == malId) {
      _selectedShow = _selectedShow!.copyWith(folderPath: path);
    }
    notifyListeners();
    return res != null && (res['is_valid'] == true);
  }

  Future<bool> migrateLibrary(String from, String to) async {
    _isMigrating = true;
    _migrationResult = null;
    notifyListeners();

    final res = await bridge.migrateLibrary(from, to);
    _isMigrating = false;
    _migrationResult = res.exitCode == 0 ? res.stdout.toString() : 'Error: ${res.stderr}';
    await refreshLocations();

    _events.insert(
      0,
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: res.exitCode == 0 ? ActivityLevel.info : ActivityLevel.error,
        message: 'Migrated library from "$from" to "$to"',
        timestamp: DateTime.now(),
      ),
    );
    notifyListeners();
    return res.exitCode == 0;
  }
}
