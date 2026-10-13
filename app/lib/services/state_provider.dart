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
  SystemStatus _status;
  List<QueueItem> _queue = [];
  List<InboxItem> _inbox = [];
  List<LibraryShow> _shows = [];
  List<ActivityItem> _events = [];
  List<RuleItem> _rules = [];
  LibraryShow? _selectedShow;
  bool _isSyncing = false;
  bool _isScanning = false;

  StateProvider({CoreBridge? bridge})
      : bridge = bridge ?? CoreBridge(),
        _status = const SystemStatus() {
    _status = this.bridge.getInitialStatus();
    _queue = this.bridge.getInitialQueue();
    _inbox = this.bridge.getInitialInbox();
    _shows = this.bridge.getInitialShows();
    _events = this.bridge.getInitialEvents();
    _rules = this.bridge.getInitialRules();
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

  int get inboxCount => _inbox.length;
  int get activeQueueCount =>
      _queue.where((i) => i.state == QueueState.downloading).length;

  // Navigation
  void selectPage(int index) {
    _selectedPageIndex = index;
    notifyListeners();
  }

  void openShowDetail(LibraryShow show) {
    _selectedShow = show;
    notifyListeners();
  }

  void closeShowDetail() {
    _selectedShow = null;
    notifyListeners();
  }

  void setSearchFilter(String query) {
    _searchFilter = query;
    notifyListeners();
  }

  // Actions
  Future<void> toggleAutomation() async {
    final nextState = !_status.automationOn;
    if (nextState) {
      await bridge.resumeQueue();
    } else {
      await bridge.pauseQueue();
    }
    _status = SystemStatus(
      daemonConnected: _status.daemonConnected,
      daemonPort: _status.daemonPort,
      automationOn: nextState,
      qbitConnected: _status.qbitConnected,
      qbitVersion: _status.qbitVersion,
      downloadSpeedBytes: nextState ? _status.downloadSpeedBytes : 0,
      uploadSpeedBytes: _status.uploadSpeedBytes,
      activeTorrents: _status.activeTorrents,
      totalShows: _status.totalShows,
      totalEpisodes: _status.totalEpisodes,
      inboxQuestions: _status.inboxQuestions,
      freeDiskSpaceGb: _status.freeDiskSpaceGb,
      totalDiskSpaceGb: _status.totalDiskSpaceGb,
      lastMalSync: _status.lastMalSync,
    );
    notifyListeners();
  }

  Future<void> forceSyncMal() async {
    _isSyncing = true;
    notifyListeners();
    await bridge.syncMal();
    _events.insert(
      0,
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Manual MAL sync triggered and completed successfully',
        timestamp: DateTime.now(),
      ),
    );
    _status = SystemStatus(
      daemonConnected: _status.daemonConnected,
      daemonPort: _status.daemonPort,
      automationOn: _status.automationOn,
      qbitConnected: _status.qbitConnected,
      qbitVersion: _status.qbitVersion,
      downloadSpeedBytes: _status.downloadSpeedBytes,
      uploadSpeedBytes: _status.uploadSpeedBytes,
      activeTorrents: _status.activeTorrents,
      totalShows: _status.totalShows,
      totalEpisodes: _status.totalEpisodes,
      inboxQuestions: _status.inboxQuestions,
      freeDiskSpaceGb: _status.freeDiskSpaceGb,
      totalDiskSpaceGb: _status.totalDiskSpaceGb,
      lastMalSync: DateTime.now(),
    );
    _isSyncing = false;
    notifyListeners();
  }

  Future<void> manualDiskScan() async {
    _isScanning = true;
    notifyListeners();
    await bridge.rescanLibrary();
    _events.insert(
      0,
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Manual disk scan finished: verified shows and indexed episodes',
        timestamp: DateTime.now(),
      ),
    );
    _isScanning = false;
    notifyListeners();
  }

  void answerQuestion(int questionId, String answerKey) {
    final itemIndex = _inbox.indexWhere((q) => q.id == questionId);
    if (itemIndex != -1) {
      final question = _inbox.removeAt(itemIndex);
      _events.insert(
        0,
        ActivityItem(
          id: DateTime.now().millisecondsSinceEpoch,
          level: ActivityLevel.info,
          message: 'Question resolved for ${question.showTitle}: answer "$answerKey"',
          timestamp: DateTime.now(),
        ),
      );
      notifyListeners();
    }
  }

  void removeQueueItem(int id) {
    _queue.removeWhere((item) => item.id == id);
    notifyListeners();
  }
}
