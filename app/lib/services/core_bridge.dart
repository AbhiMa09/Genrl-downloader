import 'dart:convert';
import 'dart:io';
import '../models/activity_item.dart';
import '../models/inbox_item.dart';
import '../models/library_item.dart';
import '../models/queue_item.dart';
import '../models/system_status.dart';

/// The Bridge between Flutter UI and the Rust core (`genrl` CLI & engine).
/// Per system design: The UI never touches the network or the disk directly;
/// it routes commands and reads states through the Rust core.
class CoreBridge {
  final String genrlBinaryPath;
  final String? customDataDir;

  CoreBridge({
    this.genrlBinaryPath = 'genrl',
    this.customDataDir,
  });

  String _resolveBinary() {
    if (genrlBinaryPath != 'genrl') return genrlBinaryPath;
    final home = Platform.environment['HOME'];
    if (home != null) {
      final localBin = '$home/.local/bin/genrl';
      if (File(localBin).existsSync()) return localBin;
      final cargoBin = '$home/.cargo/bin/genrl';
      if (File(cargoBin).existsSync()) return cargoBin;
    }
    return 'genrl';
  }

  /// Execute a `genrl` command line.
  Future<ProcessResult> execute(List<String> args) async {
    final fullArgs = <String>[];
    if (customDataDir != null && customDataDir!.isNotEmpty) {
      fullArgs.addAll(['--data-dir', customDataDir!]);
    }
    fullArgs.addAll(args);

    try {
      final bin = _resolveBinary();
      return await Process.run(bin, fullArgs);
    } catch (e) {
      // In case genrl is not in PATH during dev, fall back gracefully
      return ProcessResult(-1, 1, '', e.toString());
    }
  }

  /// Pause downloads in scheduler
  Future<bool> pauseQueue() async {
    final res = await execute(['pause']);
    return res.exitCode == 0;
  }

  /// Resume downloads in scheduler
  Future<bool> resumeQueue() async {
    final res = await execute(['resume']);
    return res.exitCode == 0;
  }

  /// Trigger MAL sync
  Future<bool> syncMal() async {
    final res = await execute(['mal', 'sync']);
    return res.exitCode == 0;
  }

  /// Trigger Library rescan
  Future<bool> rescanLibrary() async {
    final res = await execute(['library', 'rescan']);
    return res.exitCode == 0;
  }

  /// Perform candidate search on Nyaa
  Future<List<Map<String, dynamic>>> search(String query) async {
    final res = await execute(['search', query, '--json']);
    if (res.exitCode == 0) {
      try {
        final List<dynamic> decoded = jsonDecode(res.stdout as String);
        return decoded.cast<Map<String, dynamic>>();
      } catch (_) {}
    }
    return [];
  }

  /// Grab a torrent or release via CLI
  Future<bool> grab(String target, {String? to, String? category}) async {
    final args = ['grab', target];
    if (to != null) args.addAll(['--to', to]);
    if (category != null) args.addAll(['--category', category]);
    final res = await execute(args);
    return res.exitCode == 0;
  }

  /// Connect to MAL via PKCE OAuth (runs `genrl mal connect`)
  Future<ProcessResult> malConnectStart({
    String? clientId,
    String? clientSecret,
    int? port,
    String? redirectUri,
    bool noOpen = false,
  }) async {
    final args = ['mal', 'connect'];
    if (clientId != null && clientId.trim().isNotEmpty) {
      args.addAll(['--client-id', clientId.trim()]);
    }
    if (clientSecret != null && clientSecret.trim().isNotEmpty) {
      args.addAll(['--client-secret', clientSecret.trim()]);
    }
    if (port != null) {
      args.addAll(['--port', port.toString()]);
    }
    if (redirectUri != null && redirectUri.trim().isNotEmpty) {
      args.addAll(['--redirect-uri', redirectUri.trim()]);
    }
    if (noOpen) {
      args.add('--no-open');
    }
    return await execute(args);
  }

  /// Get MAL status as structured map
  Future<Map<String, dynamic>?> getMalStatus() async {
    final res = await execute(['mal', 'status', '--json']);
    if (res.exitCode == 0) {
      try {
        return jsonDecode(res.stdout as String) as Map<String, dynamic>;
      } catch (_) {}
    }
    return null;
  }

  /// Disconnect MAL credentials and public profile
  Future<bool> malDisconnect() async {
    final res = await execute(['mal', 'disconnect']);
    return res.exitCode == 0;
  }

  /// Set public MAL username for Jikan syncing
  Future<bool> malSetPublicProfile(String username) async {
    final res = await execute(['mal', 'public-profile', username]);
    return res.exitCode == 0;
  }

  /// Get rules for global or specific show
  Future<Map<String, dynamic>?> getRules({int? malId}) async {
    final args = ['rules', 'get'];
    if (malId != null) {
      args.addAll(['--mal-id', malId.toString()]);
    }
    final res = await execute(args);
    if (res.exitCode == 0) {
      try {
        return jsonDecode(res.stdout as String) as Map<String, dynamic>;
      } catch (_) {}
    }
    return null;
  }

  /// Update a single rule key
  Future<bool> updateRule(String key, dynamic value, {int? malId}) async {
    final args = ['rules', 'set', key, jsonEncode(value)];
    if (malId != null) {
      args.addAll(['--mal-id', malId.toString()]);
    }
    final res = await execute(args);
    return res.exitCode == 0;
  }

  /// Reset rule(s)
  Future<bool> resetRule({String? key, int? malId}) async {
    final args = ['rules', 'reset'];
    if (key != null) {
      args.addAll(['--key', key]);
    }
    if (malId != null) {
      args.addAll(['--mal-id', malId.toString()]);
    }
    final res = await execute(args);
    return res.exitCode == 0;
  }

  /// Set show auto download toggle
  Future<bool> setShowAuto(int malId, bool auto) async {
    return await updateRule('auto_plan_to_watch', auto, malId: malId);
  }

  /// Trigger download/planning of a specific episode
  Future<bool> downloadEpisode(int malId, int episodeNumber) async {
    final res = await execute(['plan', '--mal-id', malId.toString(), '--episode', episodeNumber.toString(), '--confirm']);
    return res.exitCode == 0;
  }

  /// Trigger download/planning of an entire season
  Future<bool> downloadSeason(int malId) async {
    final res = await execute(['plan', '--mal-id', malId.toString(), '--confirm']);
    return res.exitCode == 0;
  }

  /// Get current storage locations report
  Future<Map<String, dynamic>?> getLocations() async {
    final res = await execute(['locations', 'get', '--json']);
    if (res.exitCode == 0) {
      try {
        return jsonDecode(res.stdout as String) as Map<String, dynamic>;
      } catch (_) {}
    }
    return null;
  }

  /// Set storage location (download_dir, library_dir, or per-show folder override)
  Future<Map<String, dynamic>?> setLocation(String key, String path, {int? malId}) async {
    final args = ['locations', 'set', key, path];
    if (malId != null) {
      args.addAll(['--mal-id', malId.toString()]);
    }
    final res = await execute(args);
    if (res.exitCode == 0) {
      return await checkLocation(path);
    }
    return null;
  }

  /// Live-check directory path
  Future<Map<String, dynamic>?> checkLocation(String path) async {
    final res = await execute(['locations', 'check', path, '--json']);
    if (res.exitCode == 0) {
      try {
        return jsonDecode(res.stdout as String) as Map<String, dynamic>;
      } catch (_) {}
    }
    return null;
  }

  /// Get per-show folder override info
  Future<Map<String, dynamic>?> getShowFolder(int malId) async {
    final res = await execute(['locations', 'get-show', malId.toString(), '--json']);
    if (res.exitCode == 0) {
      try {
        return jsonDecode(res.stdout as String) as Map<String, dynamic>;
      } catch (_) {}
    }
    return null;
  }

  /// Explicit confirmed migration of existing library
  Future<ProcessResult> migrateLibrary(String from, String to) async {
    return await execute(['locations', 'migrate', '--from', from, '--to', to, '--confirm']);
  }

  /// Get live system status from core
  Future<SystemStatus?> getSystemStatus() async {
    final res = await execute(['status', '--json']);
    if (res.exitCode == 0) {
      try {
        final data = jsonDecode(res.stdout as String) as Map<String, dynamic>;
        DateTime? lastSync;
        if (data['last_mal_sync'] != null) {
          final ts = (data['last_mal_sync'] as num).toInt();
          lastSync = DateTime.fromMillisecondsSinceEpoch(ts * 1000);
        }
        return SystemStatus(
          daemonConnected: data['daemon_connected'] == true,
          daemonPort: data['daemon_port']?.toString() ?? 'CLI IPC',
          automationOn: data['automation_on'] == true,
          qbitConnected: data['qbit_connected'] == true,
          qbitVersion: data['qbit_version']?.toString() ?? 'Offline',
          downloadSpeedBytes: (data['download_speed_bytes'] as num?)?.toDouble() ?? 0.0,
          uploadSpeedBytes: (data['upload_speed_bytes'] as num?)?.toDouble() ?? 0.0,
          activeTorrents: (data['active_torrents'] as num?)?.toInt() ?? 0,
          totalShows: (data['total_shows'] as num?)?.toInt() ?? 0,
          totalEpisodes: (data['total_episodes'] as num?)?.toInt() ?? 0,
          inboxQuestions: (data['inbox_questions'] as num?)?.toInt() ?? 0,
          freeDiskSpaceGb: (data['free_disk_space_gb'] as num?)?.toInt() ?? 0,
          totalDiskSpaceGb: (data['total_disk_space_gb'] as num?)?.toInt() ?? 0,
          lastMalSync: lastSync,
        );
      } catch (_) {}
    }
    return null;
  }

  /// Get active queue items from core
  Future<List<QueueItem>> getQueue() async {
    final res = await execute(['queue', '--json']);
    if (res.exitCode == 0) {
      try {
        final List<dynamic> list = jsonDecode(res.stdout as String);
        return list.map((item) {
          final m = item as Map<String, dynamic>;
          final stateStr = m['state']?.toString().toLowerCase() ?? 'queued';
          QueueState st;
          if (stateStr == 'monitor' || stateStr == 'downloading') {
            st = QueueState.downloading;
          } else if (stateStr == 'finalize') {
            st = QueueState.finalizing;
          } else if (stateStr == 'done') {
            st = QueueState.completed;
          } else if (stateStr == 'blocked') {
            st = QueueState.blocked;
          } else if (stateStr == 'needs_input') {
            st = QueueState.needsInput;
          } else if (stateStr == 'choose' || stateStr == 'search') {
            st = QueueState.searching;
          } else {
            st = QueueState.queued;
          }
          return QueueItem(
            id: (m['id'] as num).toInt(),
            malId: (m['mal_id'] as num?)?.toInt() ?? 0,
            showTitle: m['show_title']?.toString() ?? 'Unknown Show',
            season: (m['season'] as num?)?.toInt() ?? 1,
            episode: (m['episode'] as num?)?.toInt() ?? 1,
            releaseName: m['release_name']?.toString() ?? 'Finding Release...',
            group: m['group']?.toString() ?? 'Auto',
            progress: (m['progress'] as num?)?.toDouble() ?? 0.0,
            speedBytes: 0,
            seeders: (m['seeders'] as num?)?.toInt() ?? 0,
            leechers: (m['leechers'] as num?)?.toInt() ?? 0,
            sizeBytes: (m['size_bytes'] as num?)?.toInt() ?? 0,
            etaSeconds: 0,
            state: st,
          );
        }).toList();
      } catch (_) {}
    }
    return [];
  }

  /// Get open inbox questions from core
  Future<List<InboxItem>> getInbox() async {
    final res = await execute(['inbox', '--json']);
    if (res.exitCode == 0) {
      try {
        final List<dynamic> list = jsonDecode(res.stdout as String);
        return list.map((item) {
          final m = item as Map<String, dynamic>;
          final kind = m['kind']?.toString() ?? 'match_review';
          QuestionType qt;
          List<InboxChoice> choices = [];
          if (kind == 'file_exists') {
            qt = QuestionType.fileExists;
            choices = const [
              InboxChoice(key: 'replace', label: 'Replace File', description: 'Overwrite library copy with new release'),
              InboxChoice(key: 'keep_both', label: 'Keep Both', description: 'Rename as duplicate'),
              InboxChoice(key: 'keep', label: 'Keep Existing', description: 'Discard download and keep library file'),
            ];
          } else if (kind == 'low_seeds') {
            qt = QuestionType.thinSeeds;
            choices = const [
              InboxChoice(key: 'download', label: 'Download Anyway'),
              InboxChoice(key: 'pick_another', label: 'Try Next Best Candidate'),
            ];
          } else if (kind == 'batch_only') {
            qt = QuestionType.batchOnly;
            choices = const [
              InboxChoice(key: 'batch', label: 'Download Batch'),
              InboxChoice(key: 'wait_for_singles', label: 'Wait For Singles'),
            ];
          } else {
            qt = QuestionType.matchReview;
            choices = const [
              InboxChoice(key: 'none_of_these', label: 'None of these match'),
              InboxChoice(key: 'search_again', label: 'Search again'),
            ];
          }
          final createdAtTs = (m['created_at'] as num?)?.toInt() ?? 0;
          return InboxItem(
            id: (m['id'] as num).toInt(),
            jobId: (m['job_id'] as num?)?.toInt(),
            malId: (m['mal_id'] as num?)?.toInt() ?? 0,
            showTitle: m['show_title']?.toString() ?? 'Question',
            title: 'Attention Required: $kind',
            details: m['payload']?.toString() ?? '',
            type: qt,
            choices: choices,
            createdAt: DateTime.fromMillisecondsSinceEpoch(createdAtTs * 1000),
          );
        }).toList();
      } catch (_) {}
    }
    return [];
  }

  /// Answer an inbox question
  Future<bool> answerQuestion(int questionId, String choiceKey, {bool always = false}) async {
    final args = ['answer', questionId.toString(), choiceKey];
    if (always) args.add('--always');
    final res = await execute(args);
    return res.exitCode == 0;
  }

  String _formatStatus(String? raw) {
    if (raw == null) return 'Plan to Watch';
    final lower = raw.toLowerCase().replaceAll('_', ' ').replaceAll('-', ' ').trim();
    switch (lower) {
      case 'watching':
        return 'Watching';
      case 'completed':
        return 'Completed';
      case 'on hold':
      case 'onhold':
        return 'On Hold';
      case 'dropped':
        return 'Dropped';
      case 'plan to watch':
      case 'plantowatch':
      default:
        return 'Plan to Watch';
    }
  }

  /// Get shows list from core database
  Future<List<LibraryShow>> getShows({String? status}) async {
    final args = ['shows', '--json'];
    if (status != null && status.isNotEmpty && status.toLowerCase() != 'all') {
      args.addAll(['--status', status.toLowerCase().replaceAll(' ', '_')]);
    }
    final res = await execute(args);
    if (res.exitCode == 0) {
      try {
        final List<dynamic> list = jsonDecode(res.stdout as String);
        return list.map((item) {
          final m = item as Map<String, dynamic>;
          return LibraryShow(
            malId: (m['mal_id'] as num).toInt(),
            title: m['title']?.toString() ?? '',
            englishTitle: m['english_title']?.toString(),
            startYear: (m['start_year'] as num?)?.toInt() ?? 0,
            totalEpisodes: (m['total_episodes'] as num?)?.toInt() ?? 0,
            downloadedEpisodes: (m['downloaded_episodes'] as num?)?.toInt() ?? 0,
            watchedEpisodes: (m['watched_episodes'] as num?)?.toInt() ?? 0,
            score: (m['score'] as num?)?.toDouble() ?? 0.0,
            status: _formatStatus(m['status']?.toString()),
            airingStatus: m['airing_status']?.toString() ?? 'Finished',
            genres: const [],
            synopsis: m['synopsis']?.toString() ?? '',
            posterUrl: m['poster_url']?.toString(),
            folderPath: m['folder_path']?.toString(),
            auto: m['auto'] == true,
            metadataStatus: m['metadata_status']?.toString() ?? 'ready',
            metadataError: m['metadata_error']?.toString(),
            episodes: const [],
          );
        }).toList();
      } catch (_) {}
    }
    return [];
  }

  /// Get detailed show with its episode list
  Future<LibraryShow?> getShowDetail(int malId) async {
    final res = await execute(['show', malId.toString(), '--json']);
    if (res.exitCode == 0) {
      try {
        final m = jsonDecode(res.stdout as String) as Map<String, dynamic>;
        final epList = (m['episodes'] as List<dynamic>? ?? []).map((ep) {
          final e = ep as Map<String, dynamic>;
          final epNum = (e['number'] as num).toInt();
          return EpisodeItem(
            id: (e['id'] as num?)?.toInt(),
            number: epNum,
            title: e['title']?.toString() ?? 'Episode $epNum',
            isDownloaded: e['is_downloaded'] == true,
            state: e['state']?.toString().toLowerCase() ??
                (e['is_downloaded'] == true ? 'done' : 'missing'),
            filePath: e['file_path']?.toString(),
            sizeBytes: (e['size_bytes'] as num?)?.toInt(),
            releaseName: e['release_name']?.toString(),
            group: e['group']?.toString() ?? 'Auto',
            container: e['container']?.toString() ?? 'Matroska (MKV)',
            videoTrack: e['video_track']?.toString() ?? '1080p BD Remux HEVC 10-bit',
            primaryAudio: e['primary_audio']?.toString() ?? 'FLAC 5.1ch 24-bit 48kHz',
            subtitles: e['subtitles']?.toString() ?? 'ENG (PGS), JPN',
            crc32: e['crc32']?.toString() ?? '5E2A9D11',
            chapters: (e['chapters'] as num?)?.toInt() ?? 18,
            duration: e['duration']?.toString() ?? '24:12',
          );
        }).toList();

        final aliasesList = (m['aliases'] as List<dynamic>? ?? [])
            .map((a) => a.toString())
            .toList();

        return LibraryShow(
          malId: (m['mal_id'] as num).toInt(),
          title: m['title']?.toString() ?? '',
          englishTitle: m['english_title']?.toString(),
          japaneseTitle: m['japanese_title']?.toString(),
          startYear: (m['start_year'] as num?)?.toInt() ?? 0,
          totalEpisodes: (m['total_episodes'] as num?)?.toInt() ?? 0,
          downloadedEpisodes: (m['downloaded_episodes'] as num?)?.toInt() ?? 0,
          watchedEpisodes: (m['watched_episodes'] as num?)?.toInt() ?? 0,
          score: (m['score'] as num?)?.toDouble() ?? 0.0,
          status: _formatStatus(m['status']?.toString()),
          airingStatus: m['airing_status']?.toString() ?? 'Finished',
          genres: const [],
          synopsis: m['synopsis']?.toString() ?? '',
          posterUrl: m['poster_url']?.toString(),
          folderPath: m['folder_path']?.toString(),
          auto: m['auto'] == true,
          mediaType: m['media_type']?.toString() ?? 'tv',
          season: (m['season'] as num?)?.toInt() ?? 1,
          episodeOffset: (m['episode_offset'] as num?)?.toInt() ?? 0,
          aliases: aliasesList,
          rules: m['rules'] as Map<String, dynamic>?,
          onDiskBytes: (m['on_disk_bytes'] as num?)?.toInt() ?? 0,
          metadataStatus: m['metadata_status']?.toString() ?? 'ready',
          metadataError: m['metadata_error']?.toString(),
          metadataNextRetryAt: (m['metadata_next_retry_at'] as num?)?.toInt(),
          metadataUpdatedAt: (m['metadata_updated_at'] as num?)?.toInt(),
          episodes: epList,
        );
      } catch (_) {}
    }
    return null;
  }

  /// Explicitly fetch metadata for a show (runs `genrl metadata fetch <id> [--force] [--json]`)
  Future<Map<String, dynamic>?> fetchShowMetadata(int malId, {bool force = false}) async {
    final args = ['metadata', 'fetch', malId.toString(), '--json'];
    if (force) args.add('--force');
    final res = await execute(args);
    if (res.exitCode == 0) {
      try {
        return jsonDecode(res.stdout as String) as Map<String, dynamic>;
      } catch (_) {}
    }
    return null;
  }

  /// Add an alias for a show
  Future<bool> addAlias(int malId, String alias) async {
    final res = await execute(['alias', 'add', malId.toString(), alias]);
    return res.exitCode == 0;
  }

  /// Remove an alias for a show
  Future<bool> removeAlias(int malId, String alias) async {
    final res = await execute(['alias', 'remove', malId.toString(), alias]);
    return res.exitCode == 0;
  }

  /// Set episode number offset for a show
  Future<bool> setEpisodeOffset(int malId, int offset) async {
    final res = await execute(['offset', malId.toString(), offset.toString()]);
    return res.exitCode == 0;
  }

  /// Get recent activity events from core
  Future<List<ActivityItem>> getEvents({int limit = 50, bool errorsOnly = false}) async {
    final args = ['events', '--limit', limit.toString(), '--json'];
    if (errorsOnly) args.add('--errors-only');
    final res = await execute(args);
    if (res.exitCode == 0) {
      try {
        final List<dynamic> list = jsonDecode(res.stdout as String);
        return list.map((item) {
          final m = item as Map<String, dynamic>;
          final lvlStr = m['level']?.toString().toLowerCase() ?? 'info';
          ActivityLevel lvl;
          if (lvlStr == 'error') {
            lvl = ActivityLevel.error;
          } else if (lvlStr == 'warn') {
            lvl = ActivityLevel.warn;
          } else {
            lvl = ActivityLevel.info;
          }
          final atSecs = (m['at'] as num?)?.toInt() ?? 0;
          return ActivityItem(
            id: (m['id'] as num).toInt(),
            level: lvl,
            message: m['message']?.toString() ?? '',
            timestamp: DateTime.fromMillisecondsSinceEpoch(atSecs * 1000),
            jobId: (m['job_id'] as num?)?.toInt(),
            malId: (m['mal_id'] as num?)?.toInt(),
          );
        }).toList();
      } catch (_) {}
    }
    return [];
  }
}

