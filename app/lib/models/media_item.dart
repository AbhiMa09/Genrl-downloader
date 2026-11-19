/// Machine code the CLI prints as `{"error": "tmdb_not_configured"}` (exit 0)
/// when no TMDB API key is stored yet.
const String kTmdbNotConfigured = 'tmdb_not_configured';

/// Builds the TMDB image CDN URL for a sanitized poster path.
///
/// The core only ever returns `/name.ext`, so this can never assemble an image
/// URL out of arbitrary stored data. No API key is needed for image requests.
String? tmdbPosterUrl(String? posterPath) {
  if (posterPath == null || posterPath.isEmpty) return null;
  return 'https://image.tmdb.org/t/p/w342$posterPath';
}

/// Aggregate episode states reported by `media list` for a series.
class MediaProgress {
  final int wanted;
  final int queued;
  final int done;
  final int unaired;

  const MediaProgress({
    this.wanted = 0,
    this.queued = 0,
    this.done = 0,
    this.unaired = 0,
  });

  int get total => wanted + queued + done + unaired;

  String formatSummary({int season = 1}) {
    return 'S$season · $done/$total';
  }

  factory MediaProgress.fromJson(Map<String, dynamic> json) => MediaProgress(
    wanted: (json['wanted'] as num?)?.toInt() ?? 0,
    queued: (json['queued'] as num?)?.toInt() ?? 0,
    done: (json['done'] as num?)?.toInt() ?? 0,
    unaired: (json['unaired'] as num?)?.toInt() ?? 0,
  );
}

/// One managed movie or series row from `media list --json`.
class MediaListItem {
  final int id;
  final String kind; // 'movie' | 'series'
  final String title;
  final int? year;
  final int? tmdbId;
  final String? status;
  final MediaProgress? progress; // series only
  final String? posterPath;
  final String? overview;

  const MediaListItem({
    required this.id,
    required this.kind,
    required this.title,
    this.year,
    this.tmdbId,
    this.status,
    this.progress,
    this.posterPath,
    this.overview,
  });

  bool get isSeries => kind == 'series';

  String? get posterUrl => tmdbPosterUrl(posterPath);

  String? get progressLabel => progress?.formatSummary();

  String get displayStatus {
    switch (status?.toLowerCase()) {
      case 'in_library':
      case 'completed':
        return 'In Library';
      case 'downloading':
        return 'Downloading';
      case 'queued':
        return 'Queued';
      case 'failed':
        return 'Failed';
      default:
        return status ?? 'Queued';
    }
  }

  factory MediaListItem.fromJson(Map<String, dynamic> json) => MediaListItem(
    id: (json['id'] as num?)?.toInt() ?? 0,
    kind: json['kind']?.toString() ?? 'movie',
    title: json['title']?.toString() ?? '',
    year: (json['year'] as num?)?.toInt(),
    tmdbId: (json['tmdb_id'] as num?)?.toInt(),
    status: json['status']?.toString(),
    progress: json['progress'] is Map
        ? MediaProgress.fromJson(json['progress'] as Map<String, dynamic>)
        : null,
    posterPath: json['poster_path']?.toString(),
    overview: json['overview']?.toString(),
  );
}

/// One TMDB hit from `media search --json`.
class MediaSearchItem {
  final int tmdbId;
  final String title;
  final int? year;
  final String? posterPath;
  final String? overview;

  const MediaSearchItem({
    required this.tmdbId,
    required this.title,
    this.year,
    this.posterPath,
    this.overview,
  });

  String? get posterUrl => tmdbPosterUrl(posterPath);

  factory MediaSearchItem.fromJson(Map<String, dynamic> json) =>
      MediaSearchItem(
        tmdbId: (json['tmdb_id'] as num?)?.toInt() ?? 0,
        title: json['title']?.toString() ?? '',
        year: (json['year'] as num?)?.toInt(),
        posterPath: json['poster_path']?.toString(),
        overview: json['overview']?.toString(),
      );
}

/// Receipt for `media add --json`.
class MediaAddResult {
  final int mediaId;
  final int jobsCreated;

  const MediaAddResult({required this.mediaId, required this.jobsCreated});

  factory MediaAddResult.fromJson(Map<String, dynamic> json) => MediaAddResult(
    mediaId: (json['media_id'] as num?)?.toInt() ?? 0,
    jobsCreated: (json['jobs_created'] as num?)?.toInt() ?? 0,
  );
}

/// Alias for compatibility.
typedef MediaSearchResult = MediaSearchItem;

/// One managed episode row for a series.
class MediaEpisodeItem {
  final int id;
  final int mediaId;
  final int season;
  final int episode;
  final String? airDate;
  final String state; // 'wanted', 'queued', 'done'
  final String? filePath;

  const MediaEpisodeItem({
    required this.id,
    required this.mediaId,
    required this.season,
    required this.episode,
    this.airDate,
    required this.state,
    this.filePath,
  });

  bool get isDone => state.toLowerCase() == 'done';
  bool get isQueued => state.toLowerCase() == 'queued';
  bool get isWanted => state.toLowerCase() == 'wanted';

  factory MediaEpisodeItem.fromJson(Map<String, dynamic> json) =>
      MediaEpisodeItem(
        id: (json['id'] as num?)?.toInt() ?? 0,
        mediaId: (json['media_id'] as num?)?.toInt() ?? 0,
        season: (json['season'] as num?)?.toInt() ?? 1,
        episode: (json['episode'] as num?)?.toInt() ?? 1,
        airDate: json['air_date']?.toString(),
        state: json['state']?.toString() ?? 'wanted',
        filePath: json['file_path']?.toString(),
      );
}
