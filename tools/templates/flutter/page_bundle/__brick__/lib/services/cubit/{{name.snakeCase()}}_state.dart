import '../../models/{{name.snakeCase()}}_model.dart';

/// State representation for {{name.pascalCase()}}.
class {{name.pascalCase()}}State {
  final bool isLoading;
  final List<{{name.pascalCase()}}Item> items;
  final String? error;

  const {{name.pascalCase()}}State({
    this.isLoading = false,
    this.items = const [],
    this.error,
  });

  {{name.pascalCase()}}State copyWith({
    bool? isLoading,
    List<{{name.pascalCase()}}Item>? items,
    String? error,
  }) {
    return {{name.pascalCase()}}State(
      isLoading: isLoading ?? this.isLoading,
      items: items ?? this.items,
      error: error,
    );
  }
}
