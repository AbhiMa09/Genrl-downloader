import 'dart:convert';
import 'dart:io';
import '../models/activity_item.dart';
import '../models/inbox_item.dart';
import '../models/library_item.dart';
import '../models/queue_item.dart';
import '../models/rule_item.dart';
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

  /// Execute a `genrl` command line.
  Future<ProcessResult> execute(List<String> args) async {
    final fullArgs = <String>[];
    if (customDataDir != null && customDataDir!.isNotEmpty) {
      fullArgs.addAll(['--data-dir', customDataDir!]);
    }
    fullArgs.addAll(args);

    try {
      return await Process.run(genrlBinaryPath, fullArgs);
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

  /// Provide initial mock data or database data for seamless desktop operation
  SystemStatus getInitialStatus() {
    return SystemStatus(
      daemonConnected: true,
      daemonPort: 'IPC Linked',
      automationOn: true,
      qbitConnected: true,
      qbitVersion: 'v5.2.4',
      downloadSpeedBytes: 42800000,
      uploadSpeedBytes: 1200000,
      activeTorrents: 4,
      totalShows: 1428,
      totalEpisodes: 38912,
      inboxQuestions: 3,
      freeDiskSpaceGb: 428,
      totalDiskSpaceGb: 18400,
      lastMalSync: DateTime.now().subtract(const Duration(minutes: 4)),
    );
  }

  List<QueueItem> getInitialQueue() {
    return [
      const QueueItem(
        id: 1,
        malId: 52991,
        showTitle: 'Sousou no Frieren',
        season: 1,
        episode: 28,
        releaseName: '[SubsPlease] Sousou no Frieren - 28 (1080p) [A1B2C3D4].mkv',
        group: 'SubsPlease',
        progress: 0.88,
        speedBytes: 18400000,
        seeders: 85,
        leechers: 4,
        sizeBytes: 1400000000,
        etaSeconds: 42,
        state: QueueState.downloading,
      ),
      const QueueItem(
        id: 2,
        malId: 5114,
        showTitle: 'Fullmetal Alchemist: Brotherhood',
        season: 1,
        episode: 12,
        releaseName: '[Judas] Fullmetal Alchemist Brotherhood - S01E12 [BD1080p].mkv',
        group: 'Judas',
        progress: 0.45,
        speedBytes: 14200000,
        seeders: 42,
        leechers: 2,
        sizeBytes: 950000000,
        etaSeconds: 135,
        state: QueueState.downloading,
      ),
      const QueueItem(
        id: 3,
        malId: 48583,
        showTitle: 'Shingeki no Kyojin: The Final Season',
        season: 4,
        episode: 16,
        releaseName: '[Erai-raws] Shingeki no Kyojin The Final Season - 16 [1080p].mkv',
        group: 'Erai-raws',
        progress: 0.12,
        speedBytes: 10200000,
        seeders: 120,
        leechers: 18,
        sizeBytes: 1350000000,
        etaSeconds: 310,
        state: QueueState.downloading,
      ),
      const QueueItem(
        id: 4,
        malId: 9253,
        showTitle: 'Steins;Gate',
        season: 1,
        episode: 24,
        releaseName: '[Coalgirls] Steins Gate - 24 (1920x1080 Blu-Ray FLAC).mkv',
        group: 'Coalgirls',
        progress: 1.0,
        speedBytes: 0,
        seeders: 15,
        leechers: 0,
        sizeBytes: 1200000000,
        etaSeconds: 0,
        state: QueueState.finalizing,
      ),
    ];
  }

  List<InboxItem> getInitialInbox() {
    return [
      InboxItem(
        id: 101,
        jobId: 14,
        malId: 52991,
        showTitle: 'Sousou no Frieren',
        title: 'Episode File Already Exists',
        details: 'Sousou no Frieren - S01E28.mkv already exists in the library directory (size: 1.38 GB).',
        type: QuestionType.fileExists,
        createdAt: DateTime.now().subtract(const Duration(minutes: 18)),
        choices: const [
          InboxChoice(key: 'replace', label: 'Replace File', description: 'Overwrite library copy with new release'),
          InboxChoice(key: 'keep_both', label: 'Keep Both', description: 'Rename as Sousou no Frieren - S01E28 (1).mkv'),
          InboxChoice(key: 'keep', label: 'Keep Existing', description: 'Discard download and keep library file'),
        ],
      ),
      InboxItem(
        id: 102,
        jobId: 19,
        malId: 41457,
        showTitle: '86 Eighty-Six',
        title: 'Two Equivalent Releases Found',
        details: 'Scoring tied for Episode 11: both releases are 1080p BD with healthy seeders.',
        type: QuestionType.whichCandidate,
        createdAt: DateTime.now().subtract(const Duration(minutes: 42)),
        candidateA: '[SubsPlease] 86 - Eighty Six - 11 (1080p) [28 seeds]',
        candidateB: '[Erai-raws] 86 - Eighty Six - 11 [1080p] [31 seeds]',
        choices: const [
          InboxChoice(key: 'choose_a', label: 'Choose SubsPlease'),
          InboxChoice(key: 'choose_b', label: 'Choose Erai-raws'),
        ],
      ),
      InboxItem(
        id: 103,
        jobId: 23,
        malId: 38000,
        showTitle: 'Kimetsu no Yaiba',
        title: 'Thin Seeder Count Warning',
        details: 'Highest scoring candidate has only 2 active seeders. May stall or download slowly.',
        type: QuestionType.thinSeeds,
        createdAt: DateTime.now().subtract(const Duration(hours: 1)),
        choices: const [
          InboxChoice(key: 'proceed', label: 'Download Anyway'),
          InboxChoice(key: 'skip', label: 'Try Next Best Candidate'),
        ],
      ),
    ];
  }

  List<LibraryShow> getInitialShows() {
    return [
      const LibraryShow(
        malId: 52991,
        title: 'Sousou no Frieren',
        englishTitle: "Frieren: Beyond Journey's End",
        startYear: 2023,
        totalEpisodes: 28,
        downloadedEpisodes: 28,
        watchedEpisodes: 28,
        score: 9.38,
        status: 'Completed',
        airingStatus: 'Finished',
        genres: ['Adventure', 'Drama', 'Fantasy'],
        synopsis: 'During their decade-long quest to defeat the Demon King, the party of the hero Himmel brought peace to the realm and disbanded.',
        posterUrl: 'https://cdn.myanimelist.net/images/anime/1015/138075.jpg',
        folderPath: '/mnt/anime/Sousou no Frieren (2023)',
      ),
      const LibraryShow(
        malId: 9253,
        title: 'Steins;Gate',
        englishTitle: 'Steins;Gate',
        startYear: 2011,
        totalEpisodes: 24,
        downloadedEpisodes: 24,
        watchedEpisodes: 24,
        score: 9.07,
        status: 'Completed',
        airingStatus: 'Finished',
        genres: ['Drama', 'Sci-Fi', 'Suspense'],
        synopsis: 'Self-proclaimed mad scientist Rintarou Okabe rents out a room in Akihabara, discovering how to send messages to the past.',
        posterUrl: 'https://cdn.myanimelist.net/images/anime/1935/127974.jpg',
        folderPath: '/mnt/anime/Steins;Gate (2011)',
      ),
      const LibraryShow(
        malId: 5114,
        title: 'Fullmetal Alchemist: Brotherhood',
        englishTitle: 'Fullmetal Alchemist: Brotherhood',
        startYear: 2009,
        totalEpisodes: 64,
        downloadedEpisodes: 48,
        watchedEpisodes: 30,
        score: 9.10,
        status: 'Watching',
        airingStatus: 'Finished',
        genres: ['Action', 'Adventure', 'Drama', 'Fantasy'],
        synopsis: 'After a horrific alchemy experiment goes wrong, the Elric brothers search for the Philosopher Stone to restore their bodies.',
        posterUrl: 'https://cdn.myanimelist.net/images/anime/1208/94745.jpg',
        folderPath: '/mnt/anime/Fullmetal Alchemist Brotherhood (2009)',
      ),
      const LibraryShow(
        malId: 37521,
        title: 'Vinland Saga',
        englishTitle: 'Vinland Saga',
        startYear: 2019,
        totalEpisodes: 24,
        downloadedEpisodes: 24,
        watchedEpisodes: 20,
        score: 8.75,
        status: 'Watching',
        airingStatus: 'Finished',
        genres: ['Action', 'Adventure', 'Drama'],
        synopsis: 'Young Thorfinn grew up listening to the stories of old sailors that had traveled the ocean and reached the place of legend, Vinland.',
        posterUrl: 'https://cdn.myanimelist.net/images/anime/1500/103005.jpg',
        folderPath: '/mnt/anime/Vinland Saga (2019)',
      ),
      const LibraryShow(
        malId: 41457,
        title: '86 Eighty-Six',
        englishTitle: '86 Eighty-Six',
        startYear: 2021,
        totalEpisodes: 23,
        downloadedEpisodes: 22,
        watchedEpisodes: 18,
        score: 8.27,
        status: 'Watching',
        airingStatus: 'Finished',
        genres: ['Action', 'Drama', 'Sci-Fi'],
        synopsis: 'The Republic of San Magnolia has long been besieged by the Giadian Empire army of unmanned drones known as the Legion.',
        posterUrl: 'https://cdn.myanimelist.net/images/anime/1987/117505.jpg',
        folderPath: '/mnt/anime/86 - Eighty Six (2021)',
      ),
    ];
  }

  List<ActivityItem> getInitialEvents() {
    final now = DateTime.now();
    return [
      ActivityItem(
        id: 1,
        level: ActivityLevel.info,
        message: 'Library organized: Steins;Gate S01E24 moved and desktop icon updated',
        timestamp: now.subtract(const Duration(minutes: 2)),
        jobId: 4,
        malId: 9253,
      ),
      ActivityItem(
        id: 2,
        level: ActivityLevel.info,
        message: 'Candidate chosen: [SubsPlease] Sousou no Frieren - 28 (1080p) (score 95, 85 seeds)',
        timestamp: now.subtract(const Duration(minutes: 6)),
        jobId: 1,
        malId: 52991,
      ),
      ActivityItem(
        id: 3,
        level: ActivityLevel.warn,
        message: 'Question opened: File exists in library for Sousou no Frieren S01E28',
        timestamp: now.subtract(const Duration(minutes: 18)),
        jobId: 14,
        malId: 52991,
      ),
      ActivityItem(
        id: 4,
        level: ActivityLevel.info,
        message: 'MyAnimeList list sync finished: 1,428 entries indexed, 0 diff errors',
        timestamp: now.subtract(const Duration(minutes: 25)),
      ),
      ActivityItem(
        id: 5,
        level: ActivityLevel.info,
        message: 'qBittorrent v5.2.4 WebUI session refreshed and authenticated',
        timestamp: now.subtract(const Duration(hours: 1)),
      ),
    ];
  }

  List<RuleItem> getInitialRules() {
    return const [
      RuleItem(
        id: 1,
        showTitle: 'Global Defaults',
        preferredGroup: 'SubsPlease, Erai-raws, Judas',
        preferredResolution: '1080p',
        minSeeders: 3,
        scoreMargin: 10.0,
        autoDownload: true,
        isGlobal: true,
      ),
      RuleItem(
        id: 2,
        malId: 52991,
        showTitle: 'Sousou no Frieren',
        preferredGroup: 'SubsPlease',
        preferredResolution: '1080p',
        minSeeders: 5,
        scoreMargin: 15.0,
        autoDownload: true,
      ),
      RuleItem(
        id: 3,
        malId: 9253,
        showTitle: 'Steins;Gate',
        preferredGroup: 'Coalgirls, Beatrice-Raws',
        preferredResolution: '1080p (BD)',
        minSeeders: 2,
        scoreMargin: 10.0,
        autoDownload: true,
      ),
    ];
  }
}
