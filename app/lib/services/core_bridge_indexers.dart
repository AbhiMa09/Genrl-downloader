import 'dart:convert';
import 'dart:io';

import '../models/indexer_item.dart';
import 'core_bridge.dart';
import 'core_bridge_indexer_parsers.dart';
import 'core_bridge_result.dart';

export 'core_bridge_indexer_parsers.dart';

typedef ProcessStarter = Future<Process> Function(
  String executable,
  List<String> arguments, {
  Map<String, String>? environment,
});

/// `indexers *` and `settings *-tmdb*` bindings for the Settings cards.
///
/// API keys only ever travel through [executeWithStdin]; the argv of these
/// commands never carries a key.
extension CoreBridgeIndexers on CoreBridge {
  static ProcessStarter processStarter = Process.start;

  Future<ProcessResult> executeWithStdin(
    List<String> args, {
    required String stdinData,
    Map<String, String>? environment,
  }) async {
    final fullArgs = <String>[];
    if (customDataDir != null && customDataDir!.isNotEmpty) {
      fullArgs.addAll(['--data-dir', customDataDir!]);
    }
    fullArgs.addAll(args);

    try {
      final bin = supervisor.resolveBinary();
      var env = environment;
      if (sessionMalRefreshToken != null &&
          sessionMalRefreshToken!.isNotEmpty) {
        env = Map<String, String>.from(environment ?? {});
        env['GENRL_MAL_REFRESH_TOKEN'] = sessionMalRefreshToken!;
      }
      final process = await processStarter(bin, fullArgs, environment: env);
      process.stdin.writeln(stdinData);
      await process.stdin.flush();
      await process.stdin.close();

      final stdoutFuture = process.stdout.transform(utf8.decoder).join();
      final stderrFuture = process.stderr.transform(utf8.decoder).join();
      final results = await Future.wait([stdoutFuture, stderrFuture]);
      final exitCode = await process.exitCode;

      final res = ProcessResult(process.pid, exitCode, results[0], results[1]);
      if (res.exitCode != 0) {
        final err = (res.stderr as String).trim();
        lastError = err.isNotEmpty ? err : (res.stdout as String).trim();
      } else {
        lastError = null;
      }
      return res;
    } catch (e) {
      lastError = e.toString();
      return ProcessResult(-1, 1, '', e.toString());
    }
  }

  Future<List<IndexerSummary>> listIndexers() async {
    final res = await execute(['indexers', 'list', '--json']);
    if (res.exitCode != 0) return const [];
    return parseIndexerList(stdoutOf(res));
  }

  /// Adds a Torznab indexer. [apiKey] is written to stdin, never to argv.
  Future<IndexerSummary?> addIndexer({
    required String id,
    required String name,
    required String url,
    List<String> kinds = const ['movie', 'series'],
    required String apiKey,
    bool test = false,
  }) async {
    final args = <String>['indexers', 'add', id, name, url];
    if (kinds.isNotEmpty) args.addAll(['--kinds', kinds.join(',')]);
    if (test) args.add('--test');
    args.add('--json');
    final res = await executeWithStdin(args, stdinData: apiKey);
    if (res.exitCode != 0) return null;
    return parseIndexerSummary(stdoutOf(res));
  }

  Future<bool> removeIndexer(String id) async {
    final res = await execute(['indexers', 'remove', id, '--json']);
    if (res.exitCode != 0) return false;
    return tryJsonObject(stdoutOf(res))?['ok'] == true;
  }

  Future<IndexerTestResult?> testIndexer(String id) async {
    final res = await execute(['indexers', 'test', id, '--json']);
    if (res.exitCode != 0) return null;
    return parseIndexerTest(stdoutOf(res));
  }

  Future<bool> setIndexerEnabled(String id, bool enabled) async {
    final res = await execute([
      'indexers',
      'set-enabled',
      id,
      enabled ? 'true' : 'false',
      '--json',
    ]);
    if (res.exitCode != 0) return false;
    return tryJsonObject(stdoutOf(res))?['ok'] == true;
  }

  /// Stores the TMDB key. Returns null on success, otherwise the failure.
  /// The key is written to stdin only and is never retained anywhere.
  Future<CoreCommandError?> setTmdbKey(String key) async {
    final res = await executeWithStdin([
      'settings',
      'set-tmdb-key',
    ], stdinData: key);
    if (res.exitCode == 0) return null;
    return cliFailure(res);
  }

  /// Live TMDB connectivity probe. Never throws; reports through [error].
  Future<TmdbTestOutcome> testTmdb() async {
    final res = await execute(['settings', 'test-tmdb', '--json']);
    final out = stdoutOf(res);
    final outcome = parseTmdbTest(out);
    if (outcome != null) return outcome;
    if (res.exitCode != 0) {
      return TmdbTestOutcome(ok: false, error: cliFailure(res));
    }
    return const TmdbTestOutcome(
      ok: false,
      error: CoreCommandError('bad_response', 'Unreadable TMDB test response'),
    );
  }

  /// Whether a TMDB key is currently stored. Never returns the key itself.
  Future<bool> getTmdbStatus() async {
    final res = await execute(['settings', 'tmdb-status', '--json']);
    if (res.exitCode != 0) return false;
    return parseTmdbConfigured(stdoutOf(res)) ?? false;
  }
}
