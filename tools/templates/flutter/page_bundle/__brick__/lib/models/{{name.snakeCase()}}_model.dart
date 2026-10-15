/// Data model for {{name.pascalCase()}}.
class {{name.pascalCase()}}Item {
  final int id;
  final String title;
  final String description;

  const {{name.pascalCase()}}Item({
    required this.id,
    required this.title,
    required this.description,
  });

  factory {{name.pascalCase()}}Item.fromJson(Map<String, dynamic> json) {
    return {{name.pascalCase()}}Item(
      id: json['id'] as int? ?? 0,
      title: json['title'] as String? ?? '',
      description: json['description'] as String? ?? '',
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'title': title,
    'description': description,
  };
}
