/// Request payload for {{name.pascalCase()}}.
class {{name.pascalCase()}}Request {
  final int? targetId;
  final bool json;

  const {{name.pascalCase()}}Request({
    this.targetId,
    this.json = true,
  });

  Map<String, dynamic> toJson() => {
    if (targetId != null) 'target_id': targetId,
    'json': json,
  };
}

/// Response payload for {{name.pascalCase()}}.
class {{name.pascalCase()}}Response {
  final bool success;
  final String? message;
  final Map<String, dynamic>? data;

  const {{name.pascalCase()}}Response({
    required this.success,
    this.message,
    this.data,
  });

  factory {{name.pascalCase()}}Response.fromJson(Map<String, dynamic> json) {
    return {{name.pascalCase()}}Response(
      success: json['success'] as bool? ?? true,
      message: json['message'] as String?,
      data: json['data'] as Map<String, dynamic>?,
    );
  }
}
