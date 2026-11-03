import 'dart:async';
import 'dart:io';

import '../models/activity_item.dart';
import '../models/inbox_item.dart';
import '../models/queue_item.dart';
import '../models/system_status.dart';
import '../widgets/loading/ingestion_progress.dart' show formatEta;
import 'core_bridge.dart';
import 'state/activity_state.dart';
import 'state/ingestion_state.dart';
import 'state/library_state.dart';
import 'state/queue_state.dart';
import 'state/rules_state.dart';
import 'state/settings_state.dart';
import 'state/show_detail_state.dart';
import 'state/state_context.dart';

export 'state/activity_state.dart';
export 'state/ingestion_state.dart';
export 'state/library_state.dart';
export 'state/queue_state.dart';
export 'state/rules_state.dart';
export 'state/settings_state.dart';
export 'state/show_detail_state.dart';
export 'state/state_context.dart';

class StateProvider extends StateContext
    with
        QueueStateMixin,
        ActivityStateMixin,
        LibraryStateMixin,
        ShowDetailStateMixin,
        RulesStateMixin,
        SettingsStateMixin,
        IngestionStateMixin {
  @override
  final CoreBridge bridge;

  int _selectedPageIndex = 0;
  String _searchFilter = '';
  SystemStatus _status = const SystemStatus(daemonConnected: false);
  List<InboxItem> _inbox = [];
  bool _isSyncing = false;
  String? _malSyncProgress;
  bool _isScanning = false;

  // Real Core connection & loading states
  bool _isCoreConnected = true;
  String? _coreError;
  bool _isLoadingStatus = false;
  bool _isLoadingInbox = false;
  String? _inboxError;

  // Background timer for live refresh & event stream simulation
  Timer? _pollingTimer;
  String? _queueFingerprint;
  int? _lastShowsChangedAt;
  StreamSubscription<bool>? _workerStatusSub;

  StateProvider({CoreBridge? bridge, bool startPolling = true})
    : bridge = bridge ?? CoreBridge() {
    refreshAll();
    if (startPolling) {
      _startPolling();
      // Queued jobs only move while the worker runs. It stays idle on automatic
      // work while automation is off, so starting it is always safe.
      if (!Platform.environment.containsKey('FLUTTER_TEST')) {
        this.bridge.startWorker();
      }
    }
    _workerStatusSub = this.bridge.supervisor.workerStatusStream.listen((_) {
      notifyListeners();
    });
  }

  bool _disposed = false;

  @override
  void dispose() {
    _disposed = true;
    _pollingTimer?.cancel();
    _workerStatusSub?.cancel();
    bridge.stopWorker();
    super.dispose();
  }

  @override
  void notifyListeners() {
    if (_disposed) return;
    super.notifyListeners();
  }

  void _startPolling() {
    _pollingTimer?.cancel();
    _pollingTimer = Timer.periodic(const Duration(seconds: 3), (_) {
      refreshLiveTelemetry();
    });
  }

  // Getters
  int get selectedPageIndex => _selectedPageIndex;
  String get searchFilter => _searchFilter;
  SystemStatus get status => _status;
  void setStatusForTesting(SystemStatus newStatus) {
    _status = newStatus;
    notifyListeners();
  }

  List<InboxItem> get inbox => _inbox;
  int get inboxCount => _inbox.length;
  bool get isSyncing => _isSyncing || _malSyncProgress != null;
  String? get malSyncProgress => _malSyncProgress;
  bool get isScanning => _isScanning;

  bool get isCoreConnected => _isCoreConnected;
  String? get coreError => _coreError;
  bool get isLoadingStatus => _isLoadingStatus;
  bool get isLoadingInbox => _isLoadingInbox;
  String? get inboxError => _inboxError;

  bool get isWorkerRunning => bridge.isWorkerRunning;

  Future<bool> startWorker() async {
    final ok = await bridge.startWorker();
    notifyListeners();
    return ok;
  }

  void stopWorker() {
    bridge.stopWorker();
    notifyListeners();
  }

  // Navigation
  @override
  void selectPage(int index) {
    _selectedPageIndex = index;
    selectedShow = null;
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
      refreshPlanPreview(),
      refreshSettings(),
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
        if (statusRes.showsChangedAt != null &&
            statusRes.showsChangedAt != _lastShowsChangedAt) {
          _lastShowsChangedAt = statusRes.showsChangedAt;
          unawaited(refreshShows());
        }
      } else {
        _isCoreConnected = false;
        _coreError = 'Core daemon unreachable (Process exit non-zero or genrl not found)';
      }

      if (queueRes != null) {
        setQueueFromTelemetry(queueRes);
        // A job moved: the Library and the open show follow without a click.
        final fingerprint = queueRes
            .map((q) => '${q.id}:${q.state.name}')
            .join(',');
        if (fingerprint != _queueFingerprint) {
          _queueFingerprint = fingerprint;
          unawaited(refreshShows());
          unawaited(refreshInbox());
        }
      }

      if (eventsRes != null) {
        await setEventsFromTelemetry(eventsRes);
      }
      await _pollMalSyncProgress();
    } catch (e) {
      _isCoreConnected = false;
      _coreError = e.toString();
    } finally {
      notifyListeners();
    }
  }

  @override
  Future<void> refreshStatus() async {
    _isLoadingStatus = true;
    _coreError = null;
    try {
      final res = await bridge.getSystemStatus();
      if (res != null) {
        _status = res;
        _lastShowsChangedAt = res.showsChangedAt;
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

  @override
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

  Future<void> answerInboxQuestion(
    int id,
    String choiceKey, {
    bool always = false,
  }) async {
    try {
      await bridge.answerQuestion(id, choiceKey, always: always);
    } catch (_) {}
    _inbox.removeWhere((i) => i.id == id);
    notifyListeners();
  }

  Future<void> answerQuestion(
    int questionId,
    String answerKey, {
    bool always = false,
  }) async {
    await bridge.answerQuestion(questionId, answerKey, always: always);
    await Future.wait([refreshInbox(), refreshEvents(), refreshQueue()]);
  }

  // Core Actions
  Future<void> toggleAutomation() async {
    final nextState = !_status.automationOn;
    if (nextState) {
      await bridge.resumeQueue();
    } else {
      await bridge.pauseQueue();
    }
    await refreshStatus();
  }

  @override
  Future<void> forceSyncMal() async {
    if (_isSyncing) return; // Two syncs would only race each other.
    _isSyncing = true;
    notifyListeners();
    final progressTimer = Timer.periodic(const Duration(seconds: 1), (_) {
      unawaited(_pollMalSyncProgress());
    });
    try {
      await bridge.syncMal();
    } finally {
      progressTimer.cancel();
      _malSyncProgress = null;
    }
    await Future.wait([
      refreshMalStatus(),
      refreshShows(),
      refreshEvents(),
      refreshStatus(),
    ]);
    _isSyncing = false;
    notifyListeners();
  }

  Future<void> _pollMalSyncProgress() async {
    try {
      final all = await bridge.getProgress();
      final p = all['mal_sync'];
      String? newProgress;
      if (p is Map) {
        final phase = p['phase']?.toString();
        final done = (p['done'] as num?)?.toInt() ?? 0;
        final total = (p['total'] as num?)?.toInt() ?? 0;
        final startedAt = (p['started_at'] as num?)?.toInt() ?? 0;

        if (phase == 'list') {
          newProgress = 'Fetching list...';
        } else if (phase == 'seasons' || total > 0) {
          final elapsed =
              DateTime.now().millisecondsSinceEpoch ~/ 1000 - startedAt;
          final eta = (done >= 3 && total > done && elapsed > 0)
              ? (elapsed / done * (total - done)).round()
              : null;
          if (eta != null && eta > 0) {
            newProgress = 'Seasons $done / $total · ${formatEta(eta)} left';
          } else {
            newProgress = 'Seasons $done / $total';
          }
        } else {
          newProgress = 'Syncing...';
        }
      }
      if (newProgress != _malSyncProgress) {
        _malSyncProgress = newProgress;
        notifyListeners();
      }
    } catch (_) {}
  }

  Future<void> manualDiskScan() async {
    _isScanning = true;
    notifyListeners();
    await bridge.rescanLibrary();
    await Future.wait([refreshShows(), refreshLocations(), refreshEvents()]);
    _isScanning = false;
    notifyListeners();
  }
}
