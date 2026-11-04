class SystemStatus {
  final bool daemonConnected;
  final String daemonPort;
  final bool automationOn;
  final bool qbitConnected;
  final String qbitVersion;
  final double downloadSpeedBytes;
  final double uploadSpeedBytes;
  final int activeTorrents;
  final int totalShows;
  final int totalEpisodes;
  final int inboxQuestions;
  final int freeDiskSpaceGb;
  final int totalDiskSpaceGb;
  final DateTime? lastMalSync;
  final int? showsChangedAt;
  final String? activeError;

  const SystemStatus({
    this.daemonConnected = true,
    this.daemonPort = 'IPC Linked',
    this.automationOn = true,
    this.qbitConnected = false,
    this.qbitVersion = '—',
    this.downloadSpeedBytes = 0.0,
    this.uploadSpeedBytes = 0.0,
    this.activeTorrents = 0,
    this.totalShows = 0,
    this.totalEpisodes = 0,
    this.inboxQuestions = 0,
    this.freeDiskSpaceGb = 0,
    this.totalDiskSpaceGb = 0,
    this.lastMalSync,
    this.showsChangedAt,
    this.activeError,
  });

  String get downloadSpeedFormatted => formattedDownloadSpeed;
  String get uploadSpeedFormatted {
    if (uploadSpeedBytes <= 0) return '0 B/s';
    if (uploadSpeedBytes < 1024 * 1024) {
      return '${(uploadSpeedBytes / 1024).toStringAsFixed(1)} KB/s';
    }
    return '${(uploadSpeedBytes / (1024 * 1024)).toStringAsFixed(1)} MB/s';
  }

  String get formattedDownloadSpeed {
    if (!qbitConnected || downloadSpeedBytes <= 0) return '—';
    if (downloadSpeedBytes < 1024 * 1024) {
      return '${(downloadSpeedBytes / 1024).toStringAsFixed(1)} KB/s';
    }
    return '${(downloadSpeedBytes / (1024 * 1024)).toStringAsFixed(1)} MB/s';
  }

  String get formattedDiskSpace {
    if (freeDiskSpaceGb <= 0) return '—';
    if (freeDiskSpaceGb >= 1000) {
      return '${(freeDiskSpaceGb / 1000).toStringAsFixed(1)} TB free';
    }
    return '$freeDiskSpaceGb GB free';
  }

  String get formattedLastSync {
    if (lastMalSync == null) return 'Never synced';
    final diff = DateTime.now().difference(lastMalSync!);
    if (diff.inMinutes < 1) return 'Synced just now';
    if (diff.inMinutes < 60) return 'Synced ${diff.inMinutes}m ago';
    return 'Synced ${diff.inHours}h ago';
  }
}
