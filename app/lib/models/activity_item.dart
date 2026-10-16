enum ActivityLevel { info, warn, error }

class ActivityItem {
  final int id;
  final ActivityLevel level;
  final String message;
  final DateTime timestamp;
  final int? jobId;
  final int? malId;

  const ActivityItem({
    required this.id,
    required this.level,
    required this.message,
    required this.timestamp,
    this.jobId,
    this.malId,
  });

  String get formattedTime {
    final h = timestamp.hour.toString().padLeft(2, '0');
    final m = timestamp.minute.toString().padLeft(2, '0');
    final s = timestamp.second.toString().padLeft(2, '0');
    return '$h:$m:$s';
  }
}
