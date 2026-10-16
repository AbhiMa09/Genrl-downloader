class EpisodeItem {
  final int? id;
  final int number;
  final String title;
  final bool isDownloaded;
  final String state; // done, downloading, queued, missing, failed
  final String? filePath;
  final int? sizeBytes;
  final DateTime? airedDate;
  final String? releaseName;
  final String group;
  final String container;
  final String videoTrack;
  final String primaryAudio;
  final String subtitles;
  final String crc32;
  final int chapters;
  final String duration;

  const EpisodeItem({
    this.id,
    required this.number,
    required this.title,
    required this.isDownloaded,
    this.state = 'missing',
    this.filePath,
    this.sizeBytes,
    this.airedDate,
    this.releaseName,
    this.group = 'Auto',
    this.container = 'Matroska (MKV)',
    this.videoTrack = '1080p',
    this.primaryAudio = 'Japanese',
    this.subtitles = 'English',
    this.crc32 = '-',
    this.chapters = 0,
    this.duration = '-',
  });
}

class LibraryShow {
  final int malId;
  final String title;
  final String? englishTitle;
  final String? japaneseTitle;
  final int startYear;
  final int totalEpisodes;
  final int downloadedEpisodes;
  final int watchedEpisodes;
  final double score;
  final String status; // Watching, Completed, Plan to Watch
  final String airingStatus; // Airing, Finished
  final List<String> genres;
  final String synopsis;
  final String? posterUrl;
  final String? folderPath;
  final bool auto;
  final String mediaType; // tv, movie, ova, ona, special
  final int season;
  final int episodeOffset;
  final List<String> aliases;
  final Map<String, dynamic>? rules;
  final int onDiskBytes;
  final String? studio;
  final int? rank;
  final String metadataStatus; // ready, pending, fetching, failed
  final String? metadataError;
  final int? metadataNextRetryAt;
  final int? metadataUpdatedAt;
  final List<EpisodeItem> episodes;

  int get id => malId;
  int get episodesTotal => totalEpisodes;
  int get episodesDownloaded => downloadedEpisodes;
  int get footprintBytes => onDiskBytes;

  const LibraryShow({
    required this.malId,
    required this.title,
    this.englishTitle,
    this.japaneseTitle,
    required this.startYear,
    required this.totalEpisodes,
    required this.downloadedEpisodes,
    required this.watchedEpisodes,
    required this.score,
    required this.status,
    required this.airingStatus,
    required this.genres,
    required this.synopsis,
    this.posterUrl,
    this.folderPath,
    this.auto = true,
    this.mediaType = 'tv',
    this.season = 1,
    this.episodeOffset = 0,
    this.aliases = const [],
    this.rules,
    this.onDiskBytes = 0,
    this.studio,
    this.rank,
    this.metadataStatus = 'ready',
    this.metadataError,
    this.metadataNextRetryAt,
    this.metadataUpdatedAt,
    this.episodes = const [],
  });

  LibraryShow copyWith({
    bool? auto,
    String? folderPath,
    int? episodeOffset,
    List<String>? aliases,
    Map<String, dynamic>? rules,
    List<EpisodeItem>? episodes,
    double? score,
    String? status,
    String? airingStatus,
    int? watchedEpisodes,
    int? downloadedEpisodes,
    String? synopsis,
    String? posterUrl,
    String? metadataStatus,
    String? metadataError,
    int? metadataNextRetryAt,
    int? metadataUpdatedAt,
  }) {
    return LibraryShow(
      malId: malId,
      title: title,
      englishTitle: englishTitle,
      japaneseTitle: japaneseTitle,
      startYear: startYear,
      totalEpisodes: totalEpisodes,
      downloadedEpisodes: downloadedEpisodes ?? this.downloadedEpisodes,
      watchedEpisodes: watchedEpisodes ?? this.watchedEpisodes,
      score: score ?? this.score,
      status: status ?? this.status,
      airingStatus: airingStatus ?? this.airingStatus,
      genres: genres,
      synopsis: synopsis ?? this.synopsis,
      posterUrl: posterUrl ?? this.posterUrl,
      folderPath: folderPath ?? this.folderPath,
      auto: auto ?? this.auto,
      mediaType: mediaType,
      season: season,
      episodeOffset: episodeOffset ?? this.episodeOffset,
      aliases: aliases ?? this.aliases,
      rules: rules ?? this.rules,
      onDiskBytes: onDiskBytes,
      studio: studio,
      rank: rank,
      metadataStatus: metadataStatus ?? this.metadataStatus,
      metadataError: metadataError ?? this.metadataError,
      metadataNextRetryAt: metadataNextRetryAt ?? this.metadataNextRetryAt,
      metadataUpdatedAt: metadataUpdatedAt ?? this.metadataUpdatedAt,
      episodes: episodes ?? this.episodes,
    );
  }

  double get downloadPercentage => totalEpisodes > 0
      ? (downloadedEpisodes / totalEpisodes).clamp(0.0, 1.0)
      : 0.0;
}
