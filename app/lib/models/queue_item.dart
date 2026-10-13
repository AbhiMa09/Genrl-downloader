enum QueueState {
  queued,
  searching,
  downloading,
  finalizing,
  completed,
  needsInput,
  stalled,
  blocked,
}

class QueueItem {
  final int id;
  final int malId;
  final String showTitle;
  final int season;
  final int episode;
  final String releaseName;
  final String group;
  final double progress; // 0.0 to 1.0
  final double speedBytes;
  final int seeders;
  final int leechers;
  final int sizeBytes;
  final int etaSeconds;
  final QueueState state;
  final String? infoHash;

  const QueueItem({
    required this.id,
    required this.malId,
    required this.showTitle,
    required this.season,
    required this.episode,
    required this.releaseName,
    required this.group,
    required this.progress,
    required this.speedBytes,
    required this.seeders,
    required this.leechers,
    required this.sizeBytes,
    required this.etaSeconds,
    required this.state,
    this.infoHash,
  });

  String get episodeLabel => 'S${season.toString().padLeft(2, '0')}E${episode.toString().padLeft(2, '0')}';

  String get formattedSpeed {
    if (progress >= 1.0 || speedBytes <= 0) return '0 B/s';
    if (speedBytes < 1024 * 1024) {
      return '${(speedBytes / 1024).toStringAsFixed(1)} KB/s';
    }
    return '${(speedBytes / (1024 * 1024)).toStringAsFixed(1)} MB/s';
  }

  String get formattedSize {
    if (sizeBytes < 1024 * 1024 * 1024) {
      return '${(sizeBytes / (1024 * 1024)).toStringAsFixed(0)} MB';
    }
    return '${(sizeBytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }

  String get formattedEta {
    if (progress >= 1.0) return 'Done';
    if (etaSeconds <= 0) return '--';
    if (etaSeconds < 60) return '${etaSeconds}s';
    final mins = etaSeconds ~/ 60;
    final secs = etaSeconds % 60;
    if (mins < 60) return '${mins}m ${secs}s';
    return '${mins ~/ 60}h ${mins % 60}m';
  }
}
