import '../../models/activity_item.dart';
import '../../widgets/loading/ingestion_progress.dart';
import 'state_context.dart';

mixin IngestionStateMixin on StateContext {
  IngestionStage _ingestionStage = IngestionStage.malSync;
  bool _isIngesting = false;
  bool _isIngestionPaused = false;
  bool _isIngestionBackgrounded = false;
  int _ingestionProcessedCount = 0;
  int _ingestionTotalCount = 0;
  int? _ingestionEtaSeconds;
  String? _ingestionError;
  bool _firstRunCompleted = true; // Set to true after initial run or loaded from settings

  IngestionStage get ingestionStage => _ingestionStage;
  bool get isIngesting => _isIngesting;
  bool get isIngestionPaused => _isIngestionPaused;
  bool get isIngestionBackgrounded => _isIngestionBackgrounded;
  int get ingestionProcessedCount => _ingestionProcessedCount;
  int get ingestionTotalCount => _ingestionTotalCount;
  int? get ingestionEtaSeconds => _ingestionEtaSeconds;
  String? get ingestionError => _ingestionError;
  bool get firstRunCompleted => _firstRunCompleted;

  void setFirstRunCompleted(bool value) {
    _firstRunCompleted = value;
    notifyListeners();
  }

  void pauseIngestion() {
    _isIngestionPaused = true;
    bridge.pauseQueue();
    notifyListeners();
  }

  void resumeIngestion() {
    _isIngestionPaused = false;
    bridge.resumeQueue();
    notifyListeners();
  }

  void backgroundIngestion() {
    _isIngestionBackgrounded = true;
    notifyListeners();
  }

  Future<void> retryIngestion() async {
    _ingestionError = null;
    await startIngestion();
  }

  Future<void> resetFirstRun() async {
    _firstRunCompleted = false;
    _isIngesting = false;
    _isIngestionBackgrounded = false;
    notifyListeners();
  }

  Future<void> startIngestion({
    bool automationOn = true,
    bool bypassDailyCap = false,
  }) async {
    _isIngesting = true;
    _isIngestionPaused = false;
    _ingestionError = null;
    _ingestionProcessedCount = 0;
    _ingestionTotalCount = 100;
    _ingestionEtaSeconds = 45;
    notifyListeners();

    try {
      // Stage 1: MAL Sync
      _ingestionStage = IngestionStage.malSync;
      _ingestionProcessedCount = 10;
      _ingestionEtaSeconds = 40;
      notifyListeners();

      addActivityEvent(ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Ingestion stage 1/4: MAL Sync started',
        timestamp: DateTime.now(),
      ));

      final syncOk = await bridge.syncMal();
      if (!syncOk) {
        throw Exception('Failed to synchronize MyAnimeList library');
      }

      // Stage 2: Metadata
      _ingestionStage = IngestionStage.metadata;
      _ingestionProcessedCount = 45;
      _ingestionEtaSeconds = 28;
      notifyListeners();

      addActivityEvent(ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Ingestion stage 2/4: Show metadata and poster synchronization',
        timestamp: DateTime.now(),
      ));

      // Refresh shows
      await refreshShows();

      // Stage 3: Plan Queue
      _ingestionStage = IngestionStage.planQueue;
      _ingestionProcessedCount = 80;
      _ingestionEtaSeconds = 14;
      notifyListeners();

      addActivityEvent(ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Ingestion stage 3/4: Queue planning & missing episode evaluation',
        timestamp: DateTime.now(),
      ));

      // Stage 4: Create Jobs
      _ingestionStage = IngestionStage.createJobs;
      _ingestionProcessedCount = 95;
      _ingestionEtaSeconds = 5;
      notifyListeners();

      addActivityEvent(ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Ingestion stage 4/4: Confirming initial download jobs',
        timestamp: DateTime.now(),
      ));

      // Confirm planning
      await bridge.execute(['plan', '--confirm']);

      if (!automationOn) {
        await bridge.pauseQueue();
      } else {
        await bridge.resumeQueue();
      }

      _ingestionStage = IngestionStage.completed;
      _ingestionProcessedCount = 100;
      _ingestionTotalCount = 100;
      _ingestionEtaSeconds = 0;
      _firstRunCompleted = true;
      _isIngesting = false;

      addActivityEvent(ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Ingestion pipeline completed successfully. Pipeline active.',
        timestamp: DateTime.now(),
      ));

      notifyListeners();
    } catch (e) {
      _ingestionStage = IngestionStage.failed;
      _ingestionError = e.toString();
      _isIngesting = false;
      addActivityEvent(ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.error,
        message: 'Ingestion pipeline error: $e',
        timestamp: DateTime.now(),
      ));
      notifyListeners();
    }
  }

  @override
  Future<void> refreshShows({String? status});
}
