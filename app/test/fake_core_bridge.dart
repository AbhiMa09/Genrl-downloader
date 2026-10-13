import 'dart:io';
import 'package:genrl_downloader/models/activity_item.dart';
import 'package:genrl_downloader/models/inbox_item.dart';
import 'package:genrl_downloader/models/library_item.dart';
import 'package:genrl_downloader/models/queue_item.dart';
import 'package:genrl_downloader/models/system_status.dart';
import 'package:genrl_downloader/services/core_bridge.dart';

enum FakeMode { data, empty, error }

class FakeCoreBridge extends CoreBridge {
  FakeMode mode;
  String errorMessage;

  FakeCoreBridge({
    this.mode = FakeMode.data,
    this.errorMessage = 'Connection refused by genrl core daemon',
  });

  @override
  Future<ProcessResult> execute(List<String> args) async {
    if (mode == FakeMode.error) {
      return ProcessResult(1, 1, '', errorMessage);
    }
    return ProcessResult(0, 0, '{}', '');
  }

  @override
  Future<SystemStatus?> getSystemStatus() async {
    if (mode == FakeMode.error) {
      throw Exception(errorMessage);
    }
    if (mode == FakeMode.empty) {
      return const SystemStatus(
        daemonConnected: false,
        qbitConnected: false,
        qbitVersion: 'Disconnected',
        downloadSpeedBytes: 0,
        uploadSpeedBytes: 0,
        freeDiskSpaceGb: 0,
        totalDiskSpaceGb: 0,
        activeTorrents: 0,
        totalShows: 0,
        totalEpisodes: 0,
        automationOn: false,
        inboxQuestions: 0,
      );
    }
    return const SystemStatus(
      daemonConnected: true,
      qbitConnected: true,
      qbitVersion: 'v4.6.5',
      downloadSpeedBytes: 52428800, // 50 MB/s
      uploadSpeedBytes: 2097152,   // 2 MB/s
      freeDiskSpaceGb: 400,
      totalDiskSpaceGb: 1000,
      activeTorrents: 3,
      totalShows: 12,
      totalEpisodes: 140,
      automationOn: true,
      inboxQuestions: 1,
    );
  }

  @override
  Future<List<QueueItem>> getQueue() async {
    if (mode == FakeMode.error) {
      throw Exception(errorMessage);
    }
    if (mode == FakeMode.empty) {
      return [];
    }
    return const [
      QueueItem(
        id: 1,
        malId: 52991,
        showTitle: 'Sousou no Frieren',
        season: 1,
        episode: 28,
        releaseName: '[SubsPlease] Sousou no Frieren - 28 (1080p) [ABCD1234].mkv',
        group: 'SubsPlease',
        progress: 0.72,
        speedBytes: 12582912,
        seeders: 142,
        leechers: 12,
        sizeBytes: 1468006400,
        etaSeconds: 32,
        state: QueueState.downloading,
        infoHash: 'abc123hash',
      ),
      QueueItem(
        id: 2,
        malId: 54959,
        showTitle: 'Dungeon Meshi',
        season: 1,
        episode: 24,
        releaseName: '[Erai-raws] Dungeon Meshi - 24 [1080p].mkv',
        group: 'Erai-raws',
        progress: 1.0,
        speedBytes: 0,
        seeders: 88,
        leechers: 3,
        sizeBytes: 1258291200,
        etaSeconds: 0,
        state: QueueState.completed,
        infoHash: 'def456hash',
      ),
    ];
  }

  @override
  Future<List<LibraryShow>> getShows({String? status}) async {
    if (mode == FakeMode.error) {
      throw Exception(errorMessage);
    }
    if (mode == FakeMode.empty) {
      return [];
    }
    return [
      const LibraryShow(
        malId: 52991,
        title: 'Sousou no Frieren',
        englishTitle: 'Frieren: Beyond Journey\'s End',
        startYear: 2023,
        totalEpisodes: 28,
        downloadedEpisodes: 28,
        watchedEpisodes: 28,
        score: 9.38,
        status: 'Watching',
        airingStatus: 'Finished',
        genres: ['Adventure', 'Drama', 'Fantasy'],
        synopsis: 'During their decade-long quest to defeat the Demon King...',
        posterUrl: null,
        auto: true,
      ),
      LibraryShow(
        malId: 54959,
        title: 'Dungeon Meshi',
        englishTitle: 'Delicious in Dungeon',
        startYear: 2024,
        totalEpisodes: 24,
        downloadedEpisodes: 12,
        watchedEpisodes: 10,
        score: 8.65,
        status: 'Plan to Watch',
        airingStatus: 'Finished',
        genres: ['Comedy', 'Fantasy'],
        synopsis: 'After having his sister devoured by a red dragon...',
        posterUrl: null,
        auto: false,
      ),
    ];
  }

  @override
  Future<LibraryShow?> getShowDetail(int malId) async {
    if (mode == FakeMode.error) {
      throw Exception(errorMessage);
    }
    if (mode == FakeMode.empty) {
      return null;
    }
    return const LibraryShow(
      malId: 52991,
      title: 'Sousou no Frieren',
      englishTitle: 'Frieren: Beyond Journey\'s End',
      startYear: 2023,
      totalEpisodes: 28,
      downloadedEpisodes: 28,
      watchedEpisodes: 28,
      score: 9.38,
      status: 'Watching',
      airingStatus: 'Finished',
      genres: ['Adventure', 'Drama', 'Fantasy'],
      synopsis: 'During their decade-long quest to defeat the Demon King...',
      posterUrl: null,
      folderPath: '/mnt/anime/Sousou no Frieren (2023)',
      auto: true,
    );
  }

  @override
  Future<List<InboxItem>> getInbox() async {
    if (mode == FakeMode.error) {
      throw Exception(errorMessage);
    }
    if (mode == FakeMode.empty) {
      return [];
    }
    return [
      InboxItem(
        id: 101,
        malId: 52991,
        showTitle: 'Sousou no Frieren',
        title: 'Existing File Collision: Frieren EP 12',
        details: 'File "Frieren - 12 (1080p).mkv" already exists in library.',
        jobId: 44,
        type: QuestionType.fileExists,
        createdAt: DateTime.now().subtract(const Duration(minutes: 5)),
        choices: const [
          InboxChoice(key: 'keep', label: 'Keep Existing'),
          InboxChoice(key: 'replace', label: 'Replace File'),
        ],
      ),
    ];
  }

  @override
  Future<List<ActivityItem>> getEvents({int limit = 50, bool errorsOnly = false}) async {
    if (mode == FakeMode.error) {
      throw Exception(errorMessage);
    }
    if (mode == FakeMode.empty) {
      return [];
    }
    return [
      ActivityItem(
        id: 1,
        level: ActivityLevel.info,
        message: 'Completed download: Sousou no Frieren - 28',
        timestamp: DateTime.now(),
        malId: 52991,
      ),
    ];
  }

  @override
  Future<Map<String, dynamic>?> getRules({int? malId}) async {
    if (mode == FakeMode.error) {
      throw Exception(errorMessage);
    }
    if (mode == FakeMode.empty) {
      return null;
    }
    return {
      'groups': ['SubsPlease', 'Erai-raws', 'Judas'],
      'preferred_resolution': '1080p',
      'min_resolution': '720p',
      'blocked_uploaders': ['BadUploader'],
      'seed_auto_threshold': 10,
      'seed_ask_min': 3,
      'seed_ask_max': 9,
      'move_mode': 'hardlink',
      'seed_ratio_target': 1.5,
      'auto_plan_to_watch': false,
    };
  }

  @override
  Future<Map<String, dynamic>?> getMalStatus() async {
    if (mode == FakeMode.error) {
      throw Exception(errorMessage);
    }
    if (mode == FakeMode.empty) {
      return {
        'status': 'not_connected',
        'username': null,
        'source': null,
        'counts': {
          'watching': 0,
          'completed': 0,
          'on_hold': 0,
          'dropped': 0,
          'plan_to_watch': 0,
        },
      };
    }
    return {
      'status': 'connected',
      'username': 'TestOtaku',
      'source': 'mal_oauth',
      'counts': {
        'watching': 12,
        'completed': 140,
        'on_hold': 3,
        'dropped': 2,
        'plan_to_watch': 25,
      },
    };
  }

  @override
  Future<Map<String, dynamic>?> getLocations() async {
    if (mode == FakeMode.error) {
      throw Exception(errorMessage);
    }
    if (mode == FakeMode.empty) {
      return {
        'same_filesystem': true,
        'download_dir': {'path': '', 'is_valid': false},
        'library_dir': {'path': '', 'is_valid': false},
      };
    }
    return {
      'same_filesystem': true,
      'note': 'Fast hardlink and reflink instant moves supported',
      'download_dir': {
        'path': '/mnt/media/downloads',
        'is_valid': true,
        'free_space_human': '420 GB',
      },
      'library_dir': {
        'path': '/mnt/media/anime',
        'is_valid': true,
        'free_space_human': '420 GB',
      },
    };
  }
}
