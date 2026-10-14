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
  final String? activeError;

  const SystemStatus({
    this.daemonConnected = true,
    this.daemonPort = 'IPC Linked',
    this.automationOn = true,
    this.qbitConnected = true,
    this.qbitVersion = 'v5.2.4',
    this.downloadSpeedBytes = 42800000, // 42.8 MB/s
    this.uploadSpeedBytes = 1200000,
    this.activeTorrents = 4,
    this.totalShows = 1428,
    this.totalEpisodes = 38912,
    this.inboxQuestions = 3,
    this.freeDiskSpaceGb = 428,
    this.totalDiskSpaceGb = 18400,
    this.lastMalSync,
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
    if (downloadSpeedBytes <= 0) return '0 B/s';
    if (downloadSpeedBytes < 1024 * 1024) {
      return '${(downloadSpeedBytes / 1024).toStringAsFixed(1)} KB/s';
    }
    return '${(downloadSpeedBytes / (1024 * 1024)).toStringAsFixed(1)} MB/s';
  }

  String get formattedDiskSpace {
    if (freeDiskSpaceGb >= 1000) {
      return '${(freeDiskSpaceGb / 1000).toStringAsFixed(1)} TB free';
    }
    return '$freeDiskSpaceGb GB free';
  }

  String get formattedLastSync {
    if (lastMalSync == null) return 'Synced recently';
    final diff = DateTime.now().difference(lastMalSync!);
    if (diff.inMinutes < 1) return 'Synced just now';
    if (diff.inMinutes < 60) return 'Synced ${diff.inMinutes}m ago';
    return 'Synced ${diff.inHours}h ago';
  }
}
