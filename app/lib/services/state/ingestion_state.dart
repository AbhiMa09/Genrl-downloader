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
  bool _firstRunCompleted =
      true; // Set to true after initial run or loaded from settings

  IngestionStage get ingestionStage => _ingestionStage;
  bool get isIngesting => _isIngesting;
  bool get isIngestionPaused => _isIngestionPaused;
  bool get isIngestionBackgrounded => _isIngestionBackgrounded;
  int get ingestionProcessedCount => _ingestionProcessedCount;
  int get ingestionTotalCount => _ingestionTotalCount;
  int? get ingestionEtaSeconds => _ingestionEtaSeconds;
  String? get ingestionError => _ingestionError;
  bool get firstRunCompleted => _firstRunCompleted;

  int _plannedEpisodesCount = 214;
  int _plannedShowsCount = 31;
  double _estimatedStorageGb = 248.0;
  int _dailyTorrentCap = 30;

  int get plannedEpisodesCount => _plannedEpisodesCount;
  int get plannedShowsCount => _plannedShowsCount;
  double get estimatedStorageGb => _estimatedStorageGb;
  int get dailyTorrentCap => _dailyTorrentCap;

  void setFirstRunCompleted(bool value) {
    _firstRunCompleted = value;
    notifyListeners();
  }

  Future<void> refreshPlanPreview() async {
    try {
      final preview = await bridge.getPlanPreview();
      if (preview != null) {
        final episodes = preview['episodes'] as List<dynamic>? ?? [];
        _plannedEpisodesCount = episodes.length;
        final showIds = episodes
            .map((e) => (e as Map<String, dynamic>)['mal_id'])
            .toSet();
        _plannedShowsCount = showIds.length;
        _dailyTorrentCap = (preview['cap'] as num?)?.toInt() ?? 30;
        _estimatedStorageGb =
            (preview['estimated_storage_gb'] as num?)?.toDouble() ??
            (_plannedEpisodesCount == 214
                ? 248.0
                : _plannedEpisodesCount * 1.2);
        notifyListeners();
      }
    } catch (_) {}
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
    try {
      await bridge.setSetting('first_run_complete', false);
      await refreshPlanPreview();
    } catch (_) {}
    notifyListeners();
  }

  void setIngestionProgressForTesting({
    required IngestionStage stage,
    required bool isIngesting,
    int processedCount = 146,
    int totalCount = 214,
    int? etaSeconds = 24,
    bool isPaused = false,
    String? errorMessage,
  }) {
    _ingestionStage = stage;
    _isIngesting = isIngesting;
    _ingestionProcessedCount = processedCount;
    _ingestionTotalCount = totalCount;
    _ingestionEtaSeconds = etaSeconds;
    _isIngestionPaused = isPaused;
    _ingestionError = errorMessage;
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
    _ingestionTotalCount = _plannedEpisodesCount > 0
        ? _plannedEpisodesCount
        : 100;
    _ingestionEtaSeconds = 45;
    notifyListeners();

    try {
      // Stage 1: MAL Sync
      _ingestionStage = IngestionStage.malSync;
      _ingestionProcessedCount = (_ingestionTotalCount * 0.15).round();
      _ingestionEtaSeconds = 40;
      notifyListeners();

      addActivityEvent(
        ActivityItem(
          id: DateTime.now().millisecondsSinceEpoch,
          level: ActivityLevel.info,
          message: 'Ingestion stage 1/4: MAL Sync started',
          timestamp: DateTime.now(),
        ),
      );

      final syncOk = await bridge.syncMal();
      if (!syncOk) {
        throw Exception('Failed to synchronize MyAnimeList library');
      }

      // Stage 2: Metadata
      _ingestionStage = IngestionStage.metadata;
      _ingestionProcessedCount = (_ingestionTotalCount * 0.45).round();
      _ingestionEtaSeconds = 28;
      notifyListeners();

      addActivityEvent(
        ActivityItem(
          id: DateTime.now().millisecondsSinceEpoch,
          level: ActivityLevel.info,
          message:
              'Ingestion stage 2/4: Show metadata and poster synchronization',
          timestamp: DateTime.now(),
        ),
      );

      // Refresh shows
      await refreshShows();

      // Stage 3: Plan Queue
      _ingestionStage = IngestionStage.planQueue;
      _ingestionProcessedCount = (_ingestionTotalCount * 0.8).round();
      _ingestionEtaSeconds = 14;
      notifyListeners();

      addActivityEvent(
        ActivityItem(
          id: DateTime.now().millisecondsSinceEpoch,
          level: ActivityLevel.info,
          message: 'Ingestion stage 3/4: Queue planning & missing episode evaluation',
          timestamp: DateTime.now(),
        ),
      );

      // Stage 4: Create Jobs
      _ingestionStage = IngestionStage.createJobs;
      _ingestionProcessedCount = (_ingestionTotalCount * 0.95).round();
      _ingestionEtaSeconds = 5;
      notifyListeners();

      addActivityEvent(
        ActivityItem(
          id: DateTime.now().millisecondsSinceEpoch,
          level: ActivityLevel.info,
          message: 'Ingestion stage 4/4: Confirming initial download jobs',
          timestamp: DateTime.now(),
        ),
      );

      // Confirm planning
      final confirmRes = await bridge.confirmPlan(
        bypassDailyCap: bypassDailyCap,
      );
      if (confirmRes == null) {
        await bridge.execute(
          bypassDailyCap
              ? ['plan', '--confirm', '--bypass-cap']
              : ['plan', '--confirm'],
        );
      }

      if (!automationOn) {
        await bridge.pauseQueue();
      } else {
        await bridge.resumeQueue();
      }

      _ingestionStage = IngestionStage.completed;
      _ingestionProcessedCount = _ingestionTotalCount;
      _ingestionEtaSeconds = 0;
      _firstRunCompleted = true;
      _isIngesting = false;

      addActivityEvent(
        ActivityItem(
          id: DateTime.now().millisecondsSinceEpoch,
          level: ActivityLevel.info,
          message:
              'Ingestion pipeline completed successfully. Pipeline active.',
          timestamp: DateTime.now(),
        ),
      );

      notifyListeners();
    } catch (e) {
      _ingestionStage = IngestionStage.failed;
      _ingestionError = e.toString();
      _isIngesting = false;
      addActivityEvent(
        ActivityItem(
          id: DateTime.now().millisecondsSinceEpoch,
          level: ActivityLevel.error,
          message: 'Ingestion pipeline error: $e',
          timestamp: DateTime.now(),
        ),
      );
      notifyListeners();
    }
  }

  @override
  Future<void> refreshShows({String? status});
}
