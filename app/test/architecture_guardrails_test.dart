import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

void main() {
  group('Architecture Guardrails', () {
    test('Pages must not import CoreBridge directly (must use State/Cubit)', () {
      final pagesDir = Directory('lib/pages');
      expect(
        pagesDir.existsSync(),
        isTrue,
        reason: 'lib/pages directory should exist',
      );

      final violations = <String>[];

      for (final entity in pagesDir.listSync(recursive: true)) {
        if (entity is File && entity.path.endsWith('.dart')) {
          final lines = entity.readAsLinesSync();
          for (int i = 0; i < lines.length; i++) {
            final line = lines[i].trim();
            if (line.startsWith('import ') &&
                line.contains('core_bridge.dart')) {
              violations.add('${entity.path}:${i + 1}: $line');
            }
          }
        }
      }

      expect(
        violations,
        isEmpty,
        reason:
            'Architecture violation: Pages cannot import CoreBridge directly; '
            'they must interact via StateProvider or Cubits.\n'
            'Found violations in:\n${violations.join('\n')}',
      );
    });

    test('Widgets must not import CoreBridge directly', () {
      final widgetsDir = Directory('lib/widgets');
      if (!widgetsDir.existsSync()) return;

      final violations = <String>[];

      for (final entity in widgetsDir.listSync(recursive: true)) {
        if (entity is File && entity.path.endsWith('.dart')) {
          final lines = entity.readAsLinesSync();
          for (int i = 0; i < lines.length; i++) {
            final line = lines[i].trim();
            if (line.startsWith('import ') &&
                line.contains('core_bridge.dart')) {
              violations.add('${entity.path}:${i + 1}: $line');
            }
          }
        }
      }

      expect(
        violations,
        isEmpty,
        reason:
            'Architecture violation: UI Widgets cannot import CoreBridge directly; '
            'Found violations in:\n${violations.join('\n')}',
      );
    });
  });
}
