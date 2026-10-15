import '../models/activity_item.dart';
import '../models/inbox_item.dart';
import '../models/library_item.dart';
import '../models/queue_item.dart';
import '../models/system_status.dart';

String formatShowStatus(String? raw) {
  if (raw == null) return 'Plan to Watch';
  final lower =
      raw.toLowerCase().replaceAll('_', ' ').replaceAll('-', ' ').trim();
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

SystemStatus parseSystemStatus(Map<String, dynamic> data) {
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
    downloadSpeedBytes:
        (data['download_speed_bytes'] as num?)?.toDouble() ?? 0.0,
    uploadSpeedBytes: (data['upload_speed_bytes'] as num?)?.toDouble() ?? 0.0,
    activeTorrents: (data['active_torrents'] as num?)?.toInt() ?? 0,
    totalShows: (data['total_shows'] as num?)?.toInt() ?? 0,
    totalEpisodes: (data['total_episodes'] as num?)?.toInt() ?? 0,
    inboxQuestions: (data['inbox_questions'] as num?)?.toInt() ?? 0,
    freeDiskSpaceGb: (data['free_disk_space_gb'] as num?)?.toInt() ?? 0,
    totalDiskSpaceGb: (data['total_disk_space_gb'] as num?)?.toInt() ?? 0,
    lastMalSync: lastSync,
  );
}

List<QueueItem> parseQueueItems(List<dynamic> list) {
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
}

List<InboxItem> parseInboxItems(List<dynamic> list) {
  return list.map((item) {
    final m = item as Map<String, dynamic>;
    final kind = m['kind']?.toString() ?? 'match_review';
    QuestionType qt;
    List<InboxChoice> choices = [];
    if (kind == 'file_exists') {
      qt = QuestionType.fileExists;
      choices = const [
        InboxChoice(
            key: 'replace',
            label: 'Replace File',
            description: 'Overwrite library copy with new release'),
        InboxChoice(
            key: 'keep_both',
            label: 'Keep Both',
            description: 'Rename as duplicate'),
        InboxChoice(
            key: 'keep',
            label: 'Keep Existing',
            description: 'Discard download and keep library file'),
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
}

List<LibraryShow> parseShowsList(List<dynamic> list) {
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
      status: formatShowStatus(m['status']?.toString()),
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
}

LibraryShow parseShowDetail(Map<String, dynamic> m) {
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
      primaryAudio:
          e['primary_audio']?.toString() ?? 'FLAC 5.1ch 24-bit 48kHz',
      subtitles: e['subtitles']?.toString() ?? 'ENG (PGS), JPN',
      crc32: e['crc32']?.toString() ?? '5E2A9D11',
      chapters: (e['chapters'] as num?)?.toInt() ?? 18,
      duration: e['duration']?.toString() ?? '24:12',
    );
  }).toList();

  final aliasesList =
      (m['aliases'] as List<dynamic>? ?? []).map((a) => a.toString()).toList();

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
    status: formatShowStatus(m['status']?.toString()),
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
}

List<ActivityItem> parseEventsList(List<dynamic> list) {
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
}
