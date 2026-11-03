import 'dart:convert';
import 'dart:io';

/// Planning and download calls, plus live progress of long core tasks.
mixin CoreBridgeDownloads {
  Future<ProcessResult> execute(
    List<String> args, {
    Map<String, String>? environment,
  });

  /// Live progress of a running MAL sync / metadata pass, keyed by task.
  Future<Map<String, dynamic>> getProgress() async {
    final res = await execute(['mal', 'progress']);
    if (res.exitCode == 0) {
      try {
        return jsonDecode(res.stdout as String) as Map<String, dynamic>;
      } catch (_) {}
    }
    return const {};
  }

  /// Trigger download/planning of an entire season
  Future<bool> downloadSeason(int malId) async {
    final res = await execute([
      'plan',
      '--mal-id',
      malId.toString(),
      '--confirm',
      '--json',
    ]);
    _lastQueued = _queuedFrom(res);
    return res.exitCode == 0;
  }

  int? _lastQueued;

  /// How many jobs the last season download queued, when the core reported it.
  int? get lastQueued => _lastQueued;

  int? _queuedFrom(ProcessResult res) {
    if (res.exitCode != 0) return null;
    try {
      final decoded = jsonDecode(res.stdout as String);
      return (decoded['queued'] as num?)?.toInt();
    } catch (_) {
      return null;
    }
  }

  /// Get plan dry-run preview as structured data
  Future<Map<String, dynamic>?> getPlanPreview() async {
    final res = await execute(['plan', '--dry-run', '--json']);
    if (res.exitCode == 0) {
      try {
        return jsonDecode(res.stdout as String) as Map<String, dynamic>;
      } catch (_) {}
    }
    return null;
  }

  /// Confirm initial plan or apply batch
  Future<Map<String, dynamic>?> confirmPlan({
    bool bypassDailyCap = false,
  }) async {
    final args = ['plan', '--confirm', '--json'];
    if (bypassDailyCap) {
      args.add('--bypass-cap');
    }
    final res = await execute(args);
    if (res.exitCode == 0) {
      try {
        return jsonDecode(res.stdout as String) as Map<String, dynamic>;
      } catch (_) {}
    }
    return null;
  }
}
