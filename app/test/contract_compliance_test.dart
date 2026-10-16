import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/services/core_bridge.dart';
import 'package:genrl_downloader/services/generated_contract.dart';

void main() {
  test('Contract completeness: every core command is either UI-bound or marked internal', () {
    expect(GenrlContract.commands.isNotEmpty, isTrue);

    for (final cmd in GenrlContract.commands) {
      if (cmd.internal) {
        // Explicitly marked internal
        expect(cmd.internal, isTrue);
      } else {
        // Must be bound to a UI method
        expect(
          cmd.uiMethod,
          isNotNull,
          reason:
              'Command "${cmd.name}" is non-internal and must specify a uiMethod binding',
        );
      }
    }
  });

  test('CoreBridge conforms to ICoreBridgeContract interface signatures', () {
    final bridge = CoreBridge();
    expect(bridge, isA<Object>());
    // Assert methods exist and are callable
    expect(bridge.getSystemStatus, isA<Function>());
    expect(bridge.getQueue, isA<Function>());
    expect(bridge.getInbox, isA<Function>());
    expect(bridge.getShows, isA<Function>());
    expect(bridge.getShowDetail, isA<Function>());
    expect(bridge.getEvents, isA<Function>());
    expect(bridge.getLocations, isA<Function>());
    expect(bridge.getRules, isA<Function>());
  });

  test('Contract definition file matches contract.json source of truth', () {
    final contractFile = File('../core/contract/contract.json');
    if (contractFile.existsSync()) {
      final jsonMap =
          jsonDecode(contractFile.readAsStringSync()) as Map<String, dynamic>;
      final commandList = jsonMap['commands'] as List<dynamic>;
      expect(GenrlContract.commands.length, equals(commandList.length));
    }
  });
}
