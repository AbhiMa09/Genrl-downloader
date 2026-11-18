import 'dart:convert';
import 'dart:io';

/// A command failed in a way the UI can report without leaking anything.
class CoreCommandError {
  /// Machine code, e.g. `tmdb_not_configured` or `cli_failed`.
  final String code;

  /// Human readable message, already scrubbed of secrets.
  final String message;

  const CoreCommandError(this.code, this.message);
}

/// Mirrors the core's `redact_secrets` so a URL that carries a key can never
/// reach cubit state, a log line or an exception text.
String redactSecrets(String input) {
  final pattern = RegExp(
    r'\b(api_?key|passkey|torrent_pass)=[^&\s#]+',
    caseSensitive: false,
  );
  return input.replaceAllMapped(pattern, (m) => '${m[1]}=[REDACTED]');
}

/// stdout of a completed command as text (never null).
String stdoutOf(ProcessResult res) => res.stdout as String? ?? '';

/// stderr of a completed command as text (never null).
String stderrOf(ProcessResult res) => res.stderr as String? ?? '';

/// Decodes a JSON array payload, or an empty list when the payload is not one.
List<dynamic> jsonDecodeList(String raw) {
  final decoded = jsonDecode(raw);
  if (decoded is! List) return const [];
  return decoded;
}

/// Decodes a JSON object payload, or null when the payload is not one.
Map<String, dynamic>? tryJsonObject(String raw) {
  if (raw.trim().isEmpty) return null;
  try {
    final decoded = jsonDecode(raw);
    if (decoded is Map<String, dynamic>) return decoded;
  } catch (_) {
    // Not JSON: a plain-text CLI failure handled by the exit-code path.
  }
  return null;
}

/// Reads the `{"error": "..."}` envelope the CLI prints with exit code 0.
///
/// Returns null when the payload carries no error field.
CoreCommandError? parseCliError(String stdout) {
  final obj = tryJsonObject(stdout);
  if (obj == null) return null;
  final code = obj['error'];
  if (code is! String || code.isEmpty) return null;
  final message = obj['message'];
  final text = message is String && message.isNotEmpty ? message : code;
  return CoreCommandError(code, redactSecrets(text));
}

/// Generic failure for a non-zero exit: no secret may survive into the text.
CoreCommandError cliFailure(ProcessResult res) {
  final err = stderrOf(res);
  final raw = err.trim().isNotEmpty ? err : stdoutOf(res);
  final scrubbed = redactSecrets(raw).trim();
  final text = scrubbed.isEmpty
      ? 'Command failed with exit code ${res.exitCode}'
      : scrubbed;
  return CoreCommandError('cli_failed', _cap(text, 400));
}

String _cap(String text, int max) =>
    text.length <= max ? text : '${text.substring(0, max)}…';
