import '../../models/activity_item.dart';
import 'state_context.dart';
import '../mal_auth_error.dart';

mixin SettingsStateMixin on StateContext {
  // MAL Status State
  String _malStatus = 'not_connected';
  String? _malUsername;
  String? _malSource;
  String? _malError;
  bool _isConnectingMal = false;
  bool get isConnectingMal => _isConnectingMal;
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
  bool get isMalAuthenticated => isMalConnected;

  Future<void> syncNow() => forceSyncMal();
  Future<void> loginMal() => connectMal();

  @override
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
      _malError = malAuthErrorSummary(e.toString());
      notifyListeners();
    }
  }

  Future<void> connectMal({
    String? clientId,
    String? clientSecret,
    int? port,
    String? redirectUri,
  }) async {
    if (_isConnectingMal) return;
    _isConnectingMal = true;
    _malError = null;
    notifyListeners();
    try {
      final res = await bridge.malConnectStart(
        clientId: clientId,
        clientSecret: clientSecret,
        port: port,
        redirectUri: redirectUri,
      );
      if (res.exitCode != 0) {
        _malError = malAuthErrorSummary(res.stderr.toString());
      } else if (clientId != null && clientId.isNotEmpty) {
        _malClientId = clientId;
      }
      await refreshMalStatus();
    } catch (error) {
      _malError = malAuthErrorSummary(error.toString());
    } finally {
      _isConnectingMal = false;
      notifyListeners();
    }
  }

  Future<void> disconnectMal() async {
    await bridge.malDisconnect();
    _malStatus = 'not_connected';
    _malUsername = null;
    _malSource = null;
    addActivityEvent(
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
      addActivityEvent(
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

  // Locations state
  Map<String, dynamic>? _locationsReport;
  Map<String, dynamic>? _downloadCheck;
  Map<String, dynamic>? _libraryCheck;
  bool _sameFilesystem = true;
  String _filesystemNote = '';
  bool _isMigrating = false;
  String? _migrationResult;

  Map<String, dynamic>? get locationsReport => _locationsReport;
  Map<String, dynamic>? get downloadCheck => _downloadCheck;
  Map<String, dynamic>? get libraryCheck => _libraryCheck;
  Map<String, dynamic> _settings = {};
  Map<String, dynamic> get settings => _settings;

  String? _customLibraryFolder;
  String get libraryFolder {
    if (_customLibraryFolder != null && _customLibraryFolder!.isNotEmpty) {
      return _customLibraryFolder!;
    }
    final settingPath = _settings['library_folder'] as String?;
    if (settingPath != null && settingPath.isNotEmpty) {
      return settingPath;
    }
    return _libraryCheck?['path'] as String? ?? '';
  }

  void setLibraryFolder(String path) {
    _customLibraryFolder = path;
    notifyListeners();
  }

  String get jikanBaseUrl {
    final url = _settings['jikan_base_url'] as String?;
    if (url != null && url.isNotEmpty) {
      return url;
    }
    return 'http://127.0.0.1:9992';
  }

  int get firstSyncCap {
    return (_settings['first_sync_cap'] as num?)?.toInt() ?? 25;
  }

  Future<bool> saveQbitEndpoint(String value) async {
    final endpoint = value.trim();
    final uri = Uri.tryParse(endpoint);
    if (uri == null ||
        !['http', 'https'].contains(uri.scheme) ||
        uri.host.isEmpty) {
      return false;
    }
    try {
      if (!await bridge.setSetting('qbit_url', endpoint)) return false;
      _settings = {..._settings, 'qbit_url': endpoint};
      await refreshStatus();
      notifyListeners();
      return true;
    } catch (_) {
      return false;
    }
  }

  Future<void> refreshSettings() async {
    try {
      final s = await bridge.getSettings();
      if (s != null) {
        _settings = s;
        notifyListeners();
      }
    } catch (_) {}
  }

  String get downloadFolder =>
      _downloadCheck?['path'] as String? ?? '/home/genrl/Downloads/Genrl';
  bool get sameFilesystem => _sameFilesystem;
  String get filesystemNote => _filesystemNote;
  bool get isMigrating => _isMigrating;
  String? get migrationResult => _migrationResult;

  @override
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
    addActivityEvent(
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

  Future<void> updateLocations({
    String? downloadDir,
    String? libraryDir,
  }) async {
    if (downloadDir != null && downloadDir.isNotEmpty) {
      await saveLocation('download_dir', downloadDir);
    }
    if (libraryDir != null && libraryDir.isNotEmpty) {
      await saveLocation('library_dir', libraryDir);
    }
  }

  Future<bool> migrateLibrary(String from, String to) async {
    _isMigrating = true;
    _migrationResult = null;
    notifyListeners();

    final res = await bridge.migrateLibrary(from, to);
    _isMigrating = false;
    _migrationResult = res.exitCode == 0
        ? res.stdout.toString()
        : 'Error: ${res.stderr}';
    await refreshLocations();

    addActivityEvent(
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

  @override
  Future<void> forceSyncMal();
}
