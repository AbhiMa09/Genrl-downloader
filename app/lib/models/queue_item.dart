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
  final String? magnet;
  final String? error;

  final double uploadSpeedBytes;

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
    this.magnet,
    this.error,
    this.uploadSpeedBytes = 0.0,
  });

  String get title => showTitle;
  bool get isQueued => state == QueueState.queued;
  String get downloadSpeed => formattedSpeed;
  String get uploadSpeed => uploadSpeedBytes > 0
      ? '${(uploadSpeedBytes / 1024).toStringAsFixed(1)} KB/s'
      : '0 B/s';
  String get eta => formattedEta;

  String get episodeLabel =>
      'S${season.toString().padLeft(2, '0')}E${episode.toString().padLeft(2, '0')}';

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

class CandidateItem {
  final int id;
  final String name;
  final String? group;
  final String? resolution;
  final int sizeBytes;
  final int seeders;
  final int leechers;
  final double score;
  final bool isRejected;
  final String? magnet;
  final String? infoHash;

  const CandidateItem({
    required this.id,
    required this.name,
    this.group,
    this.resolution,
    required this.sizeBytes,
    required this.seeders,
    required this.leechers,
    required this.score,
    required this.isRejected,
    this.magnet,
    this.infoHash,
  });

  factory CandidateItem.fromJson(Map<String, dynamic> json) {
    return CandidateItem(
      id: (json['id'] as num?)?.toInt() ?? 0,
      name: json['name']?.toString() ?? '',
      group: json['group']?.toString(),
      resolution: json['resolution']?.toString(),
      sizeBytes: (json['size_bytes'] as num?)?.toInt() ?? 0,
      seeders: (json['seeders'] as num?)?.toInt() ?? 0,
      leechers: (json['leechers'] as num?)?.toInt() ?? 0,
      score: (json['score'] as num?)?.toDouble() ?? 0.0,
      isRejected: json['rejected'] == true,
      magnet: json['magnet']?.toString(),
      infoHash: json['info_hash']?.toString(),
    );
  }

  String get formattedSize {
    if (sizeBytes < 1024 * 1024 * 1024) {
      return '${(sizeBytes / (1024 * 1024)).toStringAsFixed(0)} MB';
    }
    return '${(sizeBytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }
}
