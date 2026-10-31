import 'dart:convert';
import 'dart:io';

import '../models/queue_item.dart';

mixin CoreBridgeJobControls {
  Future<ProcessResult> execute(
    List<String> args, {
    Map<String, String>? environment,
  });

  /// Retry a stuck or failed job
  Future<bool> retryJob(int jobId) async {
    final res = await execute(['job', 'retry', jobId.toString()]);
    return res.exitCode == 0;
  }

  /// Cancel a job and optionally remove downloaded files
  Future<bool> cancelJob(int jobId, {bool deleteFiles = false}) async {
    final args = ['job', 'cancel', jobId.toString()];
    if (deleteFiles) args.add('--delete-files');
    final res = await execute(args);
    return res.exitCode == 0;
  }

  /// Retry all failed and blocked jobs
  Future<bool> retryAllJobs() async {
    final res = await execute(['job', 'retry-all']);
    return res.exitCode == 0;
  }

  /// Step a single job directly through the engine
  Future<bool> stepJob(int jobId) async {
    final res = await execute(['step', jobId.toString()]);
    return res.exitCode == 0;
  }

  /// Get candidates for a show
  Future<List<CandidateItem>> getCandidates(int malId) async {
    final res = await execute(['candidates', malId.toString(), '--json']);
    if (res.exitCode == 0) {
      try {
        final decoded = jsonDecode(res.stdout as String);
        if (decoded is List) {
          return decoded
              .map((c) => CandidateItem.fromJson(c as Map<String, dynamic>))
              .toList();
        }
      } catch (_) {}
    }
    return [];
  }
}
