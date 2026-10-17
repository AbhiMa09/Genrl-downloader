import 'dart:convert';
import 'dart:io';

import '../models/activity_item.dart';
import '../models/inbox_item.dart';
import '../models/library_item.dart';
import '../models/queue_item.dart';
import '../models/system_status.dart';
import 'core_bridge_parsers.dart';

/// The Bridge between Flutter UI and the Rust core (`genrl` CLI & engine).
/// Per system design: The UI never touches the network or the disk directly;
/// it routes commands and reads states through the Rust core.
class CoreBridge {
  final String genrlBinaryPath;
  final String? customDataDir;

  CoreBridge({this.genrlBinaryPath = 'genrl', this.customDataDir});

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
        final decoded = jsonDecode(res.stdout as String);
        if (decoded is List) {
          return decoded.cast<Map<String, dynamic>>();
        }
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
    final res = await execute([
      'plan',
      '--mal-id',
      malId.toString(),
      '--episode',
      episodeNumber.toString(),
      '--confirm',
    ]);
    return res.exitCode == 0;
  }

  /// Trigger download/planning of an entire season
  Future<bool> downloadSeason(int malId) async {
    final res = await execute([
      'plan',
      '--mal-id',
      malId.toString(),
      '--confirm',
    ]);
    return res.exitCode == 0;
  }

  /// Get plan dry-run preview as structured data
  Future<Map<String, dynamic>?> getPlanPreview() async {
    final res = await execute(['plan', '--dry-run', '--json']);
    if (res.exitCode == 0) {
      try {
        return jsonDecode(res.stdout as String) as Map<String, dynamic>;
      } catch (_) {}
    }
    return null;
  }

  /// Confirm initial plan or apply batch
  Future<Map<String, dynamic>?> confirmPlan({
    bool bypassDailyCap = false,
  }) async {
    final args = ['plan', '--confirm', '--json'];
    if (bypassDailyCap) {
      args.add('--bypass-cap');
    }
    final res = await execute(args);
    if (res.exitCode == 0) {
      try {
        return jsonDecode(res.stdout as String) as Map<String, dynamic>;
      } catch (_) {}
    }
    return null;
  }

  /// Get current system settings map
  Future<Map<String, dynamic>?> getSettings() async {
    final res = await execute(['settings', 'show']);
    if (res.exitCode == 0) {
      try {
        return jsonDecode(res.stdout as String) as Map<String, dynamic>;
      } catch (_) {}
    }
    return null;
  }

  /// Update a single system setting
  Future<bool> setSetting(String key, dynamic value) async {
    final res = await execute(['settings', 'set', key, jsonEncode(value)]);
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
  Future<Map<String, dynamic>?> setLocation(
    String key,
    String path, {
    int? malId,
  }) async {
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
    final res = await execute([
      'locations',
      'get-show',
      malId.toString(),
      '--json',
    ]);
    if (res.exitCode == 0) {
      try {
        return jsonDecode(res.stdout as String) as Map<String, dynamic>;
      } catch (_) {}
    }
    return null;
  }

  /// Explicit confirmed migration of existing library
  Future<ProcessResult> migrateLibrary(String from, String to) async {
    return await execute([
      'locations',
      'migrate',
      '--from',
      from,
      '--to',
      to,
      '--confirm',
    ]);
  }

  /// Get live system status from core
  Future<SystemStatus?> getSystemStatus() async {
    final res = await execute(['status', '--json']);
    if (res.exitCode == 0) {
      try {
        final data = jsonDecode(res.stdout as String) as Map<String, dynamic>;
        return parseSystemStatus(data);
      } catch (_) {}
    }
    return null;
  }

  /// Get active queue items from core
  Future<List<QueueItem>> getQueue() async {
    final res = await execute(['queue', '--json']);
    if (res.exitCode == 0) {
      try {
        final decoded = jsonDecode(res.stdout as String);
        if (decoded is List) {
          return parseQueueItems(decoded.cast<dynamic>());
        }
      } catch (_) {}
    }
    return [];
  }

  /// Get open inbox questions from core
  Future<List<InboxItem>> getInbox() async {
    final res = await execute(['inbox', '--json']);
    if (res.exitCode == 0) {
      try {
        final decoded = jsonDecode(res.stdout as String);
        if (decoded is List) {
          return parseInboxItems(decoded.cast<dynamic>());
        }
      } catch (_) {}
    }
    return [];
  }

  /// Answer an inbox question
  Future<bool> answerQuestion(
    int questionId,
    String choiceKey, {
    bool always = false,
  }) async {
    final args = ['answer', questionId.toString(), choiceKey];
    if (always) args.add('--always');
    final res = await execute(args);
    return res.exitCode == 0;
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
        final decoded = jsonDecode(res.stdout as String);
        if (decoded is List) {
          return parseShowsList(decoded.cast<dynamic>());
        }
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
        return parseShowDetail(m);
      } catch (_) {}
    }
    return null;
  }

  /// Explicitly fetch metadata for a show (runs `genrl metadata fetch <id> [--force] [--json]`)
  Future<Map<String, dynamic>?> fetchShowMetadata(
    int malId, {
    bool force = false,
  }) async {
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
  Future<List<ActivityItem>> getEvents({
    int limit = 50,
    bool errorsOnly = false,
  }) async {
    final args = ['events', '--limit', limit.toString(), '--json'];
    if (errorsOnly) args.add('--errors-only');
    final res = await execute(args);
    if (res.exitCode == 0) {
      try {
        final decoded = jsonDecode(res.stdout as String);
        if (decoded is List) {
          return parseEventsList(decoded.cast<dynamic>());
        }
      } catch (_) {}
    }
    return [];
  }
}
