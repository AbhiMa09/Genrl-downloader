class EpisodeItem {
  final int number;
  final String title;
  final bool isDownloaded;
  final String? filePath;
  final int? sizeBytes;
  final DateTime? airedDate;

  const EpisodeItem({
    required this.number,
    required this.title,
    required this.isDownloaded,
    this.filePath,
    this.sizeBytes,
    this.airedDate,
  });
}

class LibraryShow {
  final int malId;
  final String title;
  final String? englishTitle;
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
  final List<EpisodeItem> episodes;

  const LibraryShow({
    required this.malId,
    required this.title,
    this.englishTitle,
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
    this.episodes = const [],
  });

  LibraryShow copyWith({
    bool? auto,
    String? folderPath,
  }) {
    return LibraryShow(
      malId: malId,
      title: title,
      englishTitle: englishTitle,
      startYear: startYear,
      totalEpisodes: totalEpisodes,
      downloadedEpisodes: downloadedEpisodes,
      watchedEpisodes: watchedEpisodes,
      score: score,
      status: status,
      airingStatus: airingStatus,
      genres: genres,
      synopsis: synopsis,
      posterUrl: posterUrl,
      folderPath: folderPath ?? this.folderPath,
      auto: auto ?? this.auto,
      episodes: episodes,
    );
  }

  double get downloadPercentage =>
      totalEpisodes > 0 ? (downloadedEpisodes / totalEpisodes).clamp(0.0, 1.0) : 0.0;
}
