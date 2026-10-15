import 'package:flutter/foundation.dart';
import '../../models/{{name.snakeCase()}}_model.dart';
import '{{name.snakeCase()}}_state.dart';

/// Cubit / State Controller for {{name.pascalCase()}}.
/// Encapsulates state transitions and async actions away from the UI widgets.
class {{name.pascalCase()}}Cubit extends ChangeNotifier {
  {{name.pascalCase()}}State _state = const {{name.pascalCase()}}State();

  {{name.pascalCase()}}State get state => _state;

  Future<void> load() async {
    _state = _state.copyWith(isLoading: true, error: null);
    notifyListeners();

    try {
      // Async business logic / bridge invocation stub:
      await Future<void>.delayed(const Duration(milliseconds: 50));
      final sampleItems = [
        {{name.pascalCase()}}Item(id: 1, title: 'Item 1', description: 'Sample description'),
      ];
      _state = _state.copyWith(isLoading: false, items: sampleItems);
    } catch (e) {
      _state = _state.copyWith(isLoading: false, error: e.toString());
    }
    notifyListeners();
  }

  void reset() {
    _state = const {{name.pascalCase()}}State();
    notifyListeners();
  }
}
