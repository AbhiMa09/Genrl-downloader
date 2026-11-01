import 'dart:async';
import 'dart:io';

/// Platform-aware binary resolver and worker supervisor for `genrl`.
///
/// Ensures the application starts `genrl.exe` (or `genrl`) from its own bundle
/// folder without opening a console window, supervises the worker (`genrl run`)
/// using the identical `--data-dir`, and terminates the worker when the app exits.
class WorkerSupervisor {
  final String customBinaryPath;
  final String? customDataDir;
  Process? _workerProcess;
  bool _isWorkerRunning = false;
  StreamController<bool>? _workerStatusController;

  WorkerSupervisor({this.customBinaryPath = 'genrl', this.customDataDir});

  bool get isWorkerRunning => _isWorkerRunning;

  Stream<bool> get workerStatusStream {
    _workerStatusController ??= StreamController<bool>.broadcast();
    return _workerStatusController!.stream;
  }

  /// Resolve default data directory across platforms.
  static String? defaultDataDir() {
    if (Platform.isWindows) {
      final appdata = Platform.environment['APPDATA'];
      if (appdata != null && appdata.isNotEmpty) {
        return '$appdata\\Genrl Downloader';
      }
      return r'C:\Genrl Downloader';
    }
    if (Platform.environment['XDG_DATA_HOME'] != null &&
        Platform.environment['XDG_DATA_HOME']!.isNotEmpty) {
      return '${Platform.environment['XDG_DATA_HOME']}/genrl-downloader';
    }
    final home = Platform.environment['HOME'];
    return home != null ? '$home/.local/share/genrl-downloader' : null;
  }

  /// Resolve genrl executable path:
  /// 1. If customBinaryPath != 'genrl', use customBinaryPath
  /// 2. Check next to current app executable (bundle directory: genrl.exe or genrl)
  /// 3. Check ~/.local/bin/genrl and ~/.cargo/bin/genrl
  /// 4. Fallback to 'genrl' / 'genrl.exe'
  String resolveBinary() {
    if (customBinaryPath != 'genrl' && customBinaryPath.isNotEmpty) {
      return customBinaryPath;
    }

    // Check bundle directory first (same folder as running executable)
    try {
      final appDir = File(Platform.resolvedExecutable).parent.path;
      final ext = Platform.isWindows ? '.exe' : '';
      final bundleBin = Platform.isWindows
          ? '$appDir\\genrl$ext'
          : '$appDir/genrl$ext';
      if (File(bundleBin).existsSync()) {
        return bundleBin;
      }
    } catch (_) {}

    // Check Unix user directories
    final home = Platform.environment['HOME'];
    if (home != null) {
      final localBin = '$home/.local/bin/genrl';
      if (File(localBin).existsSync()) return localBin;
      final cargoBin = '$home/.cargo/bin/genrl';
      if (File(cargoBin).existsSync()) return cargoBin;
    }

    return Platform.isWindows ? 'genrl.exe' : 'genrl';
  }

  /// Start the worker (`genrl run`) with the same --data-dir.
  /// No console window is spawned on Windows.
  Future<bool> startWorker({Map<String, String>? environment}) async {
    if (_isWorkerRunning && _workerProcess != null) {
      return true;
    }

    final bin = resolveBinary();
    final args = <String>[];
    if (customDataDir != null && customDataDir!.isNotEmpty) {
      args.addAll(['--data-dir', customDataDir!]);
    }
    args.add('run');

    try {
      final proc = await Process.start(
        bin,
        args,
        environment: environment,
        runInShell: false,
        mode: ProcessStartMode.normal,
      );

      _workerProcess = proc;
      _isWorkerRunning = true;
      _workerStatusController?.add(true);

      // Handle worker process exit
      proc.exitCode.then((exitCode) {
        if (_workerProcess == proc) {
          _isWorkerRunning = false;
          _workerProcess = null;
          _workerStatusController?.add(false);
        }
      });

      return true;
    } catch (_) {
      _isWorkerRunning = false;
      _workerProcess = null;
      _workerStatusController?.add(false);
      return false;
    }
  }

  /// Terminate the supervised worker process.
  void stopWorker() {
    if (_workerProcess != null) {
      try {
        _workerProcess!.kill();
      } catch (_) {}
      _workerProcess = null;
    }
    _isWorkerRunning = false;
    _workerStatusController?.add(false);
  }

  void dispose() {
    stopWorker();
    _workerStatusController?.close();
  }
}
