// Error strings below are fixtures, not live service results.
import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/widgets/loading/ingestion_error_card.dart';
import 'package:genrl_downloader/widgets/loading/ingestion_progress.dart';

void main() {
  IngestionErrorInfo classify(String raw) =>
      IngestionErrorInfo.from(stage: IngestionStage.metadata, rawError: raw);

  test('timeout is not reported as a rate limit', () {
    final info = classify('Jikan request timed out after 20000 ms');
    expect(info.failureSummary, 'Jikan request timed out');
    expect(info.failureSummary, isNot(contains('429')));
  });
  test('actual rate limit retains its HTTP class', () {
    expect(
      classify('rate limited by jikan (429)').failureSummary,
      contains('HTTP 429'),
    );
  });
  test('server failure retains its status', () {
    expect(
      classify('http 503 from https://api.jikan.moe').failureSummary,
      contains('HTTP 503'),
    );
  });
  test('breaker pause is identified separately', () {
    expect(
      classify('circuit breaker open for jikan').failureSummary,
      contains('paused'),
    );
  });
  test('show identifiers are not interpreted as HTTP statuses', () {
    expect(
      classify('show #429 and show #503 unresolved').failureSummary,
      'Jikan metadata request failed',
    );
  });
  test('generic persistent failure does not invent HTTP status', () {
    expect(
      classify('Jikan persistent failure').failureSummary,
      'Jikan metadata request failed',
    );
  });
}
