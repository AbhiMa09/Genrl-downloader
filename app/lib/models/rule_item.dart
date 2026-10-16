class RuleItem {
  final int id;
  final int? malId;
  final String showTitle;
  final List<String> groups;
  final String preferredResolution; // 1080p, 720p, etc.
  final String minResolution;
  final List<String> blockedUploaders;
  final int seedAutoThreshold;
  final int seedAskMin;
  final int seedAskMax;
  final bool? autoPlanToWatch;
  final String? moveMode; // hardlink, move, copy
  final double? seedRatioTarget;
  final bool isGlobal;

  const RuleItem({
    required this.id,
    this.malId,
    required this.showTitle,
    this.groups = const [],
    this.preferredResolution = '1080p',
    this.minResolution = '720p',
    this.blockedUploaders = const [],
    this.seedAutoThreshold = 10,
    this.seedAskMin = 3,
    this.seedAskMax = 9,
    this.autoPlanToWatch,
    this.moveMode = 'hardlink',
    this.seedRatioTarget = 1.0,
    this.isGlobal = false,
  });

  String get preferredGroup =>
      groups.isNotEmpty ? groups.join(', ') : 'Any / Highest seed';

  factory RuleItem.fromJson(
    Map<String, dynamic> json, {
    int? malId,
    String? showTitle,
  }) {
    final groupsRaw = json['groups'] as List<dynamic>? ?? [];
    final blockedRaw = json['blocked_uploaders'] as List<dynamic>? ?? [];

    return RuleItem(
      id: malId ?? 0,
      malId: malId,
      showTitle:
          showTitle ??
          (malId == null ? 'Global Default Rules' : 'Show #$malId'),
      groups: groupsRaw.map((e) => e.toString()).toList(),
      preferredResolution: json['preferred_resolution'] as String? ?? '1080p',
      minResolution: json['min_resolution'] as String? ?? '720p',
      blockedUploaders: blockedRaw.map((e) => e.toString()).toList(),
      seedAutoThreshold: (json['seed_auto_threshold'] as num?)?.toInt() ?? 10,
      seedAskMin: (json['seed_ask_min'] as num?)?.toInt() ?? 3,
      seedAskMax: (json['seed_ask_max'] as num?)?.toInt() ?? 9,
      autoPlanToWatch: json['auto_plan_to_watch'] as bool?,
      moveMode: json['move_mode'] as String? ?? 'hardlink',
      seedRatioTarget: (json['seed_ratio_target'] as num?)?.toDouble() ?? 1.0,
      isGlobal: malId == null,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'groups': groups,
      'preferred_resolution': preferredResolution,
      'min_resolution': minResolution,
      'blocked_uploaders': blockedUploaders,
      'seed_auto_threshold': seedAutoThreshold,
      'seed_ask_min': seedAskMin,
      'seed_ask_max': seedAskMax,
      if (autoPlanToWatch != null) 'auto_plan_to_watch': autoPlanToWatch,
      if (moveMode != null) 'move_mode': moveMode,
      if (seedRatioTarget != null) 'seed_ratio_target': seedRatioTarget,
    };
  }
}
