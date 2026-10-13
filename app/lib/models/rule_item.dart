class RuleItem {
  final int id;
  final int? malId;
  final String showTitle;
  final String preferredGroup;
  final String preferredResolution; // 1080p, 720p, etc.
  final int minSeeders;
  final double scoreMargin;
  final bool autoDownload;
  final bool isGlobal;

  const RuleItem({
    required this.id,
    this.malId,
    required this.showTitle,
    required this.preferredGroup,
    required this.preferredResolution,
    required this.minSeeders,
    required this.scoreMargin,
    required this.autoDownload,
    this.isGlobal = false,
  });
}
