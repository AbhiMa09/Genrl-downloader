import 'dart:io';
import 'dart:math';

String formatBytes(int? bytes) {
  if (bytes == null || bytes <= 0) return '0 B';
  const suffixes = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
  var i = (log(bytes) / log(1024)).floor();
  if (i < 0) i = 0;
  if (i >= suffixes.length) i = suffixes.length - 1;
  return '${(bytes / pow(1024, i)).toStringAsFixed(2)} ${suffixes[i]}';
}

void openFileOrFolder(String? path) {
  if (path == null || path.isEmpty) return;
  if (Platform.environment.containsKey('FLUTTER_TEST')) return;
  try {
    if (Platform.isLinux) {
      Process.run('xdg-open', [path]);
    } else if (Platform.isMacOS) {
      Process.run('open', [path]);
    } else if (Platform.isWindows) {
      Process.run('explorer.exe', [path]);
    }
  } catch (_) {}
}
