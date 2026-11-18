import '../models/indexer_item.dart';
import 'core_bridge_result.dart';

List<IndexerSummary> parseIndexerList(String stdout) {
  final trimmed = stdout.trim();
  if (!trimmed.startsWith('[')) return const [];
  try {
    final decoded = jsonDecodeList(trimmed);
    return decoded
        .whereType<Map<String, dynamic>>()
        .map(IndexerSummary.fromJson)
        .toList(growable: false);
  } catch (_) {
    return const [];
  }
}

IndexerSummary? parseIndexerSummary(String stdout) {
  final obj = tryJsonObject(stdout);
  if (obj == null || obj['id'] == null) return null;
  return IndexerSummary.fromJson(obj);
}

IndexerTestResult? parseIndexerTest(String stdout) {
  final obj = tryJsonObject(stdout);
  if (obj == null || obj['ok'] == null) return null;
  return IndexerTestResult.fromJson(obj);
}

/// Result of `settings test-tmdb --json`.
class TmdbTestOutcome {
  final bool ok;
  final CoreCommandError? error;

  const TmdbTestOutcome({required this.ok, this.error});
}

/// `settings test-tmdb --json` answers `{"ok": true}` or
/// `{"ok": false, "error": "tmdb_not_configured"}`.
TmdbTestOutcome? parseTmdbTest(String stdout) {
  final obj = tryJsonObject(stdout);
  if (obj == null) return null;
  final ok = obj['ok'];
  if (ok is! bool) return null;
  if (ok) return const TmdbTestOutcome(ok: true);
  final error = obj['error'];
  final message = obj['message'];
  final code = error is String && error.isNotEmpty ? error : 'tmdb_failed';
  final text = message is String && message.isNotEmpty ? message : code;
  return TmdbTestOutcome(
    ok: false,
    error: CoreCommandError(code, redactSecrets(text)),
  );
}

/// `settings tmdb-status --json` answers `{"configured": bool}`.
bool? parseTmdbConfigured(String stdout) {
  final obj = tryJsonObject(stdout);
  if (obj == null) return null;
  final configured = obj['configured'];
  if (configured is! bool) return null;
  return configured;
}
