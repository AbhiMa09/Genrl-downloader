import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Verify zero mock placeholders exist across app/lib source files', () {
    final libDir = Directory('lib');
    expect(libDir.existsSync(), isTrue, reason: 'lib directory must exist');

    final forbiddenStrings = [
      'Tokyo Toshokan',
      'AnimeTosho',
      'AniList',
      'Sonarr',
      'CRC32: 0x9B4E1A',
      '85.4 MB/s',
      'Worker Threads: 08',
      'qBt 4.6.2',
      'v2.4.1 Linux x86_64',
      'Arch Linux',
      '/run/genrl/daemon.sock',
      'socket/port check',
      '5E2A9D11',
    ];

    final violations = <String>[];

    for (final file in libDir.listSync(recursive: true)) {
      if (file is File && file.path.endsWith('.dart')) {
        final content = file.readAsStringSync();
        for (final forbidden in forbiddenStrings) {
          if (content.contains(forbidden)) {
            violations.add(
              '${file.path}: contains forbidden placeholder "$forbidden"',
            );
          }
        }
      }
    }

    expect(
      violations,
      isEmpty,
      reason:
          'Forbidden mock placeholder strings found in app/lib:\n${violations.join('\n')}',
    );
  });
}
