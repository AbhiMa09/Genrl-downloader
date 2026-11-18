import '../models/media_item.dart';
import 'core_bridge_result.dart';

/// Outcome of `media search --json`: hits, or a machine error code.
class MediaSearchOutcome {
  final List<MediaSearchItem> items;
  final CoreCommandError? error;

  const MediaSearchOutcome({this.items = const [], this.error});

  bool get ok => error == null;
}

/// Outcome of `media add --json`: the receipt, or a machine error code.
class MediaAddOutcome {
  final MediaAddResult? result;
  final CoreCommandError? error;

  const MediaAddOutcome({this.result, this.error});

  bool get ok => error == null && result != null;
}

List<MediaSearchItem> parseMediaSearch(String stdout) {
  final trimmed = stdout.trim();
  if (!trimmed.startsWith('[')) return const [];
  try {
    final decoded = jsonDecodeList(trimmed);
    return decoded
        .whereType<Map<String, dynamic>>()
        .map(MediaSearchItem.fromJson)
        .toList(growable: false);
  } catch (_) {
    return const [];
  }
}

List<MediaListItem> parseMediaList(String stdout) {
  final trimmed = stdout.trim();
  if (!trimmed.startsWith('[')) return const [];
  try {
    final decoded = jsonDecodeList(trimmed);
    return decoded
        .whereType<Map<String, dynamic>>()
        .map(MediaListItem.fromJson)
        .toList(growable: false);
  } catch (_) {
    return const [];
  }
}

MediaAddResult? parseMediaAdd(String stdout) {
  final obj = tryJsonObject(stdout);
  if (obj == null) return null;
  if (obj['media_id'] == null) return null;
  return MediaAddResult.fromJson(obj);
}
