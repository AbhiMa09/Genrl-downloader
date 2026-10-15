import 'dart:convert';
import 'dart:io';
import '../core_bridge.dart';
import '../../models/{{name.snakeCase()}}_payload.dart';

/// Extension on [CoreBridge] to invoke the `{{name.snakeCase()}}` core command.
extension {{name.pascalCase()}}BridgeCall on CoreBridge {
  Future<{{name.pascalCase()}}Response> invoke{{name.pascalCase()}}({{name.pascalCase()}}Request request) async {
    final args = <String>['{{name.paramCase()}}'];
    if (request.json) args.add('--json');
    if (request.targetId != null) args.addAll(['--id', request.targetId.toString()]);

    final res = await execute(args);
    if (res.exitCode != 0) {
      return {{name.pascalCase()}}Response(
        success: false,
        message: res.stderr.toString().isNotEmpty ? res.stderr.toString() : 'Execution failed',
      );
    }

    try {
      if (res.stdout.toString().trim().isEmpty) {
        return const {{name.pascalCase()}}Response(success: true);
      }
      final decoded = jsonDecode(res.stdout.toString());
      if (decoded is Map<String, dynamic>) {
        return {{name.pascalCase()}}Response.fromJson(decoded);
      }
      return const {{name.pascalCase()}}Response(success: true);
    } catch (e) {
      return {{name.pascalCase()}}Response(
        success: false,
        message: 'JSON parse error: $e',
      );
    }
  }
}
