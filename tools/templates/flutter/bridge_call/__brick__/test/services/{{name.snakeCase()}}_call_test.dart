import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/models/{{name.snakeCase()}}_payload.dart';
import 'package:genrl_downloader/services/core_bridge.dart';
import 'package:genrl_downloader/services/calls/{{name.snakeCase()}}_call.dart';

void main() {
  test('{{name.pascalCase()}}Request serializes correctly', () {
    const req = {{name.pascalCase()}}Request(targetId: 42, json: true);
    final json = req.toJson();
    expect(json['target_id'], 42);
    expect(json['json'], true);
  });

  test('{{name.pascalCase()}}Response parses from map', () {
    final res = {{name.pascalCase()}}Response.fromJson({
      'success': true,
      'message': 'done',
      'data': {'count': 10},
    });
    expect(res.success, isTrue);
    expect(res.message, 'done');
    expect(res.data?['count'], 10);
  });
}
