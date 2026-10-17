import 'dart:async';

import '../models/activity_item.dart';
import '../models/inbox_item.dart';
import '../models/queue_item.dart';
import '../models/system_status.dart';
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
  bool _isScanning = false;

  // Real Core connection & loading states
  bool _isCoreConnected = true;
  String? _coreError;
  bool _isLoadingStatus = false;
  bool _isLoadingInbox = false;
  String? _inboxError;

  // Background timer for live refresh & event stream simulation
  Timer? _pollingTimer;

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
      refreshLiveTelemetry();
    });
  }

  // Getters
  int get selectedPageIndex => _selectedPageIndex;
  String get searchFilter => _searchFilter;
  SystemStatus get status => _status;
  List<InboxItem> get inbox => _inbox;
  int get inboxCount => _inbox.length;
  bool get isSyncing => _isSyncing;
  bool get isScanning => _isScanning;

  bool get isCoreConnected => _isCoreConnected;
  String? get coreError => _coreError;
  bool get isLoadingStatus => _isLoadingStatus;
  bool get isLoadingInbox => _isLoadingInbox;
  String? get inboxError => _inboxError;

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
        setQueueFromTelemetry(queueRes);
      }

      if (eventsRes != null) {
        await setEventsFromTelemetry(eventsRes);
      }
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
    await Future.wait([refreshShows(), refreshLocations(), refreshEvents()]);
    _isScanning = false;
    notifyListeners();
  }
}
