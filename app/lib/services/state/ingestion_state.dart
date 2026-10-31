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

  int _plannedEpisodesCount = 11;
  int _plannedShowsCount = 11;
  double _estimatedStorageGb = 15.4;
  int _dailyTorrentCap = 25;
  List<Map<String, dynamic>> _plannedEpisodesList = [];

  int get plannedEpisodesCount => _plannedEpisodesCount;
  int get plannedShowsCount => _plannedShowsCount;
  double get estimatedStorageGb => _estimatedStorageGb;
  int get dailyTorrentCap => _dailyTorrentCap;
  List<Map<String, dynamic>> get plannedEpisodesList => _plannedEpisodesList;

  Future<void> Function(IngestionStage stage)? onStageCompleted;
  bool _isPlanReadyAwaitingConfirm = false;
  bool get isPlanReadyAwaitingConfirm => _isPlanReadyAwaitingConfirm;
  set isPlanReadyAwaitingConfirm(bool val) {
    _isPlanReadyAwaitingConfirm = val;
    notifyListeners();
  }

  void setFirstRunCompleted(bool value) {
    _firstRunCompleted = value;
    notifyListeners();
  }

  void setIngestionError(String error, {IngestionStage? stage}) {
    _failedStage = stage ?? _ingestionStage;
    _ingestionStage = IngestionStage.failed;
    _ingestionError = error;
    notifyListeners();
  }

  void clearIngestionError({
    IngestionStage? resumeStage,
    int processed = 0,
    int total = 0,
  }) {
    _ingestionStage = resumeStage ?? IngestionStage.completed;
    _isIngesting =
        (resumeStage != null && resumeStage != IngestionStage.completed);
    if (processed > 0 || total > 0) {
      _ingestionProcessedCount = processed;
      _ingestionTotalCount = total;
    }
    _ingestionError = null;
    _failedStage = null;
    notifyListeners();
  }

  Future<void> refreshPlanPreview() async {
    try {
      final preview = await bridge.getPlanPreview();
      if (preview != null) {
        final episodes = (preview['episodes'] as List<dynamic>? ?? [])
            .map((e) => Map<String, dynamic>.from(e as Map))
            .toList();
        _plannedEpisodesList = episodes;
        _plannedEpisodesCount = episodes.length;
        final showIds = episodes.map((e) => e['mal_id']).toSet();
        _plannedShowsCount = showIds.length;
        _dailyTorrentCap = (preview['cap'] as num?)?.toInt() ?? 25;
        _estimatedStorageGb =
            (preview['estimated_storage_gb'] as num?)?.toDouble() ??
            ((_plannedEpisodesCount * 1.4 * 10).round() / 10.0);
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

  IngestionStage? _failedStage;
  IngestionStage? get failedStage => _failedStage;

  void backgroundIngestion() {
    _isIngestionBackgrounded = true;
    notifyListeners();
  }

  Future<void> retryIngestion({
    bool automationOn = true,
    bool bypassDailyCap = false,
  }) async {
    final resumeStage = _failedStage ?? IngestionStage.malSync;
    _ingestionError = null;
    await startIngestion(
      automationOn: automationOn,
      bypassDailyCap: bypassDailyCap,
      resumeFrom: resumeStage,
    );
  }

  Future<void> resetFirstRun() async {
    _firstRunCompleted = false;
    _isIngesting = false;
    _isIngestionBackgrounded = false;
    _failedStage = null;
    try {
      await bridge.setSetting('first_run_complete', false);
      await refreshPlanPreview();
    } catch (_) {}
    notifyListeners();
  }

  void setIngestionProgressForTesting({
    required IngestionStage stage,
    required bool isIngesting,
    int processedCount = 0,
    int totalCount = 0,
    int? etaSeconds,
    bool isPaused = false,
    String? errorMessage,
    IngestionStage? failedStage,
  }) {
    _ingestionStage = stage;
    _isIngesting = isIngesting;
    _ingestionProcessedCount = processedCount;
    _ingestionTotalCount = totalCount;
    _ingestionEtaSeconds = etaSeconds;
    _isIngestionPaused = isPaused;
    _ingestionError = errorMessage;
    if (stage == IngestionStage.failed) {
      _failedStage = failedStage ?? IngestionStage.createJobs;
    }
    notifyListeners();
  }

  Future<void> startIngestion({
    bool automationOn = true,
    bool bypassDailyCap = false,
    IngestionStage resumeFrom = IngestionStage.malSync,
  }) async {
    _isIngesting = true;
    _isIngestionPaused = false;
    _ingestionError = null;
    if (resumeFrom == IngestionStage.malSync) {
      _ingestionProcessedCount = 0;
    }
    _ingestionTotalCount = _plannedEpisodesCount > 0
        ? _plannedEpisodesCount
        : 100;
    _ingestionEtaSeconds = 45;
    notifyListeners();

    try {
      // Stage 1: MAL Sync
      if (resumeFrom.index <= IngestionStage.malSync.index) {
        _ingestionStage = IngestionStage.malSync;
        _ingestionProcessedCount = 0;
        _ingestionTotalCount = 1040;
        _ingestionEtaSeconds = null;
        notifyListeners();

        addActivityEvent(
          ActivityItem(
            id: DateTime.now().millisecondsSinceEpoch,
            level: ActivityLevel.info,
            message: 'Ingestion stage 1/4: MAL Sync started',
            timestamp: DateTime.now(),
          ),
        );

        if (shows.isEmpty) {
          await refreshShows();
        }
        final syncRes = await bridge.execute(['mal', 'sync']);
        if (syncRes.exitCode != 0) {
          final err = (syncRes.stderr as String).trim();
          final out = (syncRes.stdout as String).trim();
          throw Exception(
            err.isNotEmpty
                ? err
                : (out.isNotEmpty
                      ? out
                      : 'Failed to synchronize MyAnimeList library (exit code ${syncRes.exitCode})'),
          );
        }
        await refreshShows();
        _ingestionProcessedCount = shows.isNotEmpty ? shows.length : 1040;
        _ingestionTotalCount = _ingestionProcessedCount;
        notifyListeners();
        if (onStageCompleted != null) {
          await onStageCompleted!(IngestionStage.malSync);
        }
      }

      // Stage 2: Metadata (Watching shows first: n/21)
      if (resumeFrom.index <= IngestionStage.metadata.index) {
        _ingestionStage = IngestionStage.metadata;
        await refreshShows();
        final watchingShows = shows
            .where((s) => s.status.toLowerCase() == 'watching')
            .toList();
        final watchingCount = watchingShows.length;
        final readyCount = watchingShows
            .where((s) => s.metadataResolved)
            .length;
        _ingestionProcessedCount = readyCount > 0 ? readyCount : 0;
        _ingestionTotalCount = watchingCount;
        _ingestionEtaSeconds = null;
        notifyListeners();

        addActivityEvent(
          ActivityItem(
            id: DateTime.now().millisecondsSinceEpoch,
            level: ActivityLevel.info,
            message:
                'Ingestion stage 2/4: Resolving watching metadata ($watchingCount shows). Remaining shows continue in background.',
            timestamp: DateTime.now(),
          ),
        );

        final alreadyResolved = readyCount == watchingCount;
        if (!alreadyResolved) {
          final metaRes = await bridge.execute([
            'metadata',
            'sync',
            '--limit',
            watchingCount.toString(),
            '--json',
          ]);
          if (metaRes.exitCode != 0) {
            final err = (metaRes.stderr as String).trim();
            throw Exception(
              err.isNotEmpty
                  ? err
                  : 'Failed to resolve show metadata (exit code ${metaRes.exitCode})',
            );
          }
        }

        await refreshShows();

        final resolvedWatching = shows
            .where(
              (s) => s.status.toLowerCase() == 'watching' && s.metadataResolved,
            )
            .length;

        final watchingWithErrors = shows
            .where(
              (s) =>
                  s.status.toLowerCase() == 'watching' &&
                  !s.metadataResolved &&
                  s.metadataError != null &&
                  s.metadataError!.isNotEmpty,
            )
            .toList();

        _ingestionProcessedCount = resolvedWatching;
        if (resolvedWatching < watchingCount) {
          final firstErr = watchingWithErrors.isNotEmpty
              ? watchingWithErrors.first.metadataError!
              : 'Metadata is unresolved. Retry metadata or connect MAL for fallback.';
          throw Exception(
            'Metadata fetch failed for watching shows: $firstErr',
          );
        }

        _ingestionProcessedCount = resolvedWatching;
        notifyListeners();
        if (onStageCompleted != null) {
          await onStageCompleted!(IngestionStage.metadata);
        }
      }

      // Stage 3: Plan Queue (Shows evaluated)
      if (resumeFrom.index <= IngestionStage.planQueue.index) {
        _ingestionStage = IngestionStage.planQueue;
        await refreshPlanPreview();
        final showsEvaluated = _plannedShowsCount > 0 ? _plannedShowsCount : 11;
        _ingestionProcessedCount = showsEvaluated;
        _ingestionTotalCount = showsEvaluated;
        _ingestionEtaSeconds = null;
        notifyListeners();

        addActivityEvent(
          ActivityItem(
            id: DateTime.now().millisecondsSinceEpoch,
            level: ActivityLevel.info,
            message: 'Ingestion stage 3/4: Queue planning & missing episode evaluation completed',
            timestamp: DateTime.now(),
          ),
        );

        if (onStageCompleted != null) {
          await onStageCompleted!(IngestionStage.planQueue);
        }

        if (resumeFrom.index < IngestionStage.createJobs.index) {
          _isPlanReadyAwaitingConfirm = true;
          _isIngesting = false;
          notifyListeners();
          return;
        }
      }

      // Stage 4: Create Jobs (Jobs created / planned)
      if (resumeFrom.index <= IngestionStage.createJobs.index) {
        _isPlanReadyAwaitingConfirm = false;
        _isIngesting = true;
        _ingestionStage = IngestionStage.createJobs;
        final plannedJobs = _plannedEpisodesCount > 0
            ? _plannedEpisodesCount
            : 16;
        _ingestionProcessedCount = 0;
        _ingestionTotalCount = plannedJobs;
        _ingestionEtaSeconds = null;
        notifyListeners();

        addActivityEvent(
          ActivityItem(
            id: DateTime.now().millisecondsSinceEpoch,
            level: ActivityLevel.info,
            message: 'Ingestion stage 4/4: Confirming initial download jobs',
            timestamp: DateTime.now(),
          ),
        );

        final confirmRes = await bridge.confirmPlan(
          bypassDailyCap: bypassDailyCap,
        );
        if (confirmRes == null) {
          final errStr = (bridge.lastError ?? '').trim();
          throw Exception(
            errStr.isNotEmpty
                ? errStr
                : 'Failed to confirm plan and create download jobs',
          );
        }
        final qItems = await bridge.getQueue();
        final actualJobs = qItems.isNotEmpty ? qItems.length : plannedJobs;
        _ingestionProcessedCount = actualJobs;
        _ingestionTotalCount = actualJobs;
        await refreshQueue();
        await refreshStatus();
        if (onStageCompleted != null) {
          await onStageCompleted!(IngestionStage.createJobs);
        }
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
      _failedStage = null;

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
      _failedStage = _ingestionStage;
      _ingestionStage = IngestionStage.failed;
      _ingestionError = e.toString().replaceFirst(
        RegExp(r'^Exception:\s*'),
        '',
      );
      _isIngesting = false;
      addActivityEvent(
        ActivityItem(
          id: DateTime.now().millisecondsSinceEpoch,
          level: ActivityLevel.error,
          message: 'Ingestion pipeline error: $_ingestionError',
          timestamp: DateTime.now(),
        ),
      );
      notifyListeners();
    }
  }

  @override
  Future<void> refreshShows({String? status});
}
