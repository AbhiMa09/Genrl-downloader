/// One configured Torznab indexer from `indexers list --json`.
///
/// The API key is never part of this payload: the CLI exposes only [hasKey].
class IndexerSummary {
  final String id;
  final String name;
  final String baseUrl;
  final bool enabled;
  final List<String> kinds;
  final bool hasKey;
  final String? lastTestStatus;
  final int? lastTestAt;

  const IndexerSummary({
    required this.id,
    required this.name,
    required this.baseUrl,
    required this.enabled,
    required this.kinds,
    required this.hasKey,
    this.lastTestStatus,
    this.lastTestAt,
  });

  factory IndexerSummary.fromJson(Map<String, dynamic> json) => IndexerSummary(
    id: json['id']?.toString() ?? '',
    name: json['name']?.toString() ?? '',
    baseUrl: json['base_url']?.toString() ?? '',
    enabled: json['enabled'] == true,
    kinds:
        (json['kinds'] as List?)
            ?.map((k) => k.toString())
            .toList(growable: false) ??
        const [],
    hasKey: json['has_key'] == true,
    lastTestStatus: json['last_test_status']?.toString(),
    lastTestAt: (json['last_test_at'] as num?)?.toInt(),
  );

  bool get isHealthy => enabled && lastTestStatus == 'ok';
}

/// Connectivity probe result from `indexers test --json`.
class IndexerTestResult {
  final bool ok;
  final int categoriesCount;
  final bool movieSearch;
  final bool tvSearch;
  final String? error;

  const IndexerTestResult({
    required this.ok,
    this.categoriesCount = 0,
    this.movieSearch = false,
    this.tvSearch = false,
    this.error,
  });

  factory IndexerTestResult.fromJson(Map<String, dynamic> json) =>
      IndexerTestResult(
        ok: json['ok'] == true,
        categoriesCount: (json['categories_count'] as num?)?.toInt() ?? 0,
        movieSearch: json['movie_search'] == true,
        tvSearch: json['tv_search'] == true,
        error: json['error']?.toString(),
      );
}
