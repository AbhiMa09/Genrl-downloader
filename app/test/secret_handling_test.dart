import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/services/core_bridge.dart';
import 'package:genrl_downloader/services/core_bridge_indexers.dart';
import 'package:genrl_downloader/state/indexer_cubit.dart';

class _FakeStdin implements IOSink {
  final StringBuffer buffer = StringBuffer();
  final Completer<void> _doneCompleter = Completer<void>();

  @override
  Encoding encoding = utf8;

  @override
  void add(List<int> data) {
    buffer.write(utf8.decode(data));
  }

  @override
  void write(Object? object) {
    buffer.write(object);
  }

  @override
  void writeln([Object? object = '']) {
    buffer.writeln(object);
  }

  @override
  void writeAll(Iterable<dynamic> objects, [String separator = '']) {
    buffer.writeAll(objects, separator);
  }

  @override
  void writeCharCode(int charCode) {
    buffer.writeCharCode(charCode);
  }

  @override
  Future<void> addStream(Stream<List<int>> stream) async {
    await for (final chunk in stream) {
      add(chunk);
    }
  }

  @override
  Future<void> flush() async {}

  @override
  Future<void> close() async {
    if (!_doneCompleter.isCompleted) {
      _doneCompleter.complete();
    }
  }

  @override
  Future<void> get done => _doneCompleter.future;

  @override
  void addError(Object error, [StackTrace? stackTrace]) {}
}

class _FakeProcess implements Process {
  @override
  final int pid = 12345;
  @override
  // ignore: close_sinks
  final _FakeStdin stdin = _FakeStdin();
  final StreamController<List<int>> _stdoutController;
  final StreamController<List<int>> _stderrController;
  final int _exitCode;

  _FakeProcess({this._exitCode = 0, String stdout = '', String stderr = ''})
    : _stdoutController = StreamController<List<int>>(),
      _stderrController = StreamController<List<int>>() {
    if (stdout.isNotEmpty) {
      _stdoutController.add(utf8.encode(stdout));
    }
    _stdoutController.close();

    if (stderr.isNotEmpty) {
      _stderrController.add(utf8.encode(stderr));
    }
    _stderrController.close();
  }

  @override
  Stream<List<int>> get stdout => _stdoutController.stream;

  @override
  Stream<List<int>> get stderr => _stderrController.stream;

  @override
  Future<int> get exitCode async => _exitCode;

  @override
  bool kill([ProcessSignal signal = ProcessSignal.sigterm]) => true;
}

void main() {
  final originalStarter = CoreBridgeIndexers.processStarter;

  tearDown(() {
    CoreBridgeIndexers.processStarter = originalStarter;
  });

  group('Secret Handling Guardrails', () {
    test('setTmdbKey pipes secret strictly via stdin, never in argv', () async {
      final capturedArgs = <List<String>>[];
      final capturedStdin = <String>[];

      CoreBridgeIndexers.processStarter =
          (
            String executable,
            List<String> arguments, {
            Map<String, String>? environment,
          }) async {
            capturedArgs.add(List.from(arguments));
            final proc = _FakeProcess(exitCode: 0, stdout: '{"ok": true}');
            proc.stdin.done.then((_) {
              capturedStdin.add(proc.stdin.buffer.toString());
            });
            return proc;
          };

      final bridge = CoreBridge();
      const secretKey = 'TEST_SECRET_TMDB_API_KEY_xyz987';

      final err = await bridge.setTmdbKey(secretKey);
      expect(err, isNull);

      expect(capturedArgs, isNotEmpty);
      final lastArgs = capturedArgs.last;
      for (final arg in lastArgs) {
        expect(
          arg,
          isNot(contains(secretKey)),
          reason: 'Secret must never appear in command line arguments',
        );
      }

      expect(capturedStdin, isNotEmpty);
      expect(
        capturedStdin.last,
        contains(secretKey),
        reason: 'Secret must be delivered via stdin',
      );
    });

    test('addIndexer pipes apiKey strictly via stdin, never in argv', () async {
      final capturedArgs = <List<String>>[];
      final capturedStdin = <String>[];

      CoreBridgeIndexers.processStarter =
          (
            String executable,
            List<String> arguments, {
            Map<String, String>? environment,
          }) async {
            capturedArgs.add(List.from(arguments));
            final proc = _FakeProcess(
              exitCode: 0,
              stdout: jsonEncode({
                'id': 'idx_test',
                'name': 'Test Indexer',
                'base_url': 'https://indexer.example',
                'enabled': true,
                'kinds': ['movie', 'series'],
                'has_key': true,
              }),
            );
            proc.stdin.done.then((_) {
              capturedStdin.add(proc.stdin.buffer.toString());
            });
            return proc;
          };

      final bridge = CoreBridge();
      const secretApiKey = 'SUPER_CONFIDENTIAL_INDEXER_KEY_abc123';

      final result = await bridge.addIndexer(
        id: 'idx_test',
        name: 'Test Indexer',
        url: 'https://indexer.example',
        apiKey: secretApiKey,
      );

      expect(result, isNotNull);
      expect(result!.id, equals('idx_test'));
      expect(result.hasKey, isTrue);

      expect(capturedArgs, isNotEmpty);
      for (final arg in capturedArgs.last) {
        expect(
          arg,
          isNot(contains(secretApiKey)),
          reason: 'Indexer API key must never appear in CLI arguments',
        );
      }

      expect(capturedStdin, isNotEmpty);
      expect(
        capturedStdin.last,
        contains(secretApiKey),
        reason: 'Indexer API key must be piped via stdin',
      );
    });

    test('IndexerCubit state never retains or leaks API keys', () async {
      CoreBridgeIndexers.processStarter =
          (
            String executable,
            List<String> arguments, {
            Map<String, String>? environment,
          }) async {
            return _FakeProcess(exitCode: 0, stdout: '{"ok": true}');
          };

      final bridge = CoreBridge();
      final cubit = IndexerCubit(bridge: bridge);

      const secretKey = 'KEY_THAT_MUST_NEVER_EXIST_IN_STATE';
      await cubit.setTmdbKey(secretKey);

      expect(cubit.state.tmdbError, isNot(contains(secretKey)));
      expect(cubit.state.indexersError, isNot(contains(secretKey)));
      expect(cubit.state.toString(), isNot(contains(secretKey)));

      await cubit.addIndexer(
        id: 'idx1',
        name: 'Idx',
        url: 'https://idx.com',
        apiKey: secretKey,
      );

      expect(cubit.state.toString(), isNot(contains(secretKey)));
      cubit.dispose();
    });
  });
}
