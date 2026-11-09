import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/models/inbox_item.dart';
import 'package:genrl_downloader/pages/inbox/inbox_dynamic_card.dart';
import 'package:genrl_downloader/services/core_bridge_parsers.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/theme/theme.dart';

import 'fake_core_bridge.dart';

void main() {
  test('core_bridge_parsers parses season_map question payload correctly', () {
    final raw = [
      {
        'id': 1926,
        'job_id': 619,
        'mal_id': 235,
        'show_title': 'Detective Conan',
        'kind': 'season_map',
        'payload': {
          'mal_id': 235,
          'season_no': 33,
          'release_title':
              '[Group] Detective Conan Season 33 (2023) [Batch] [1080p]',
          'suggested_first': 1050,
          'suggested_last': 1093,
          'reason': 'Release specifies Season 33, which is not mapped',
        },
        'created_at': 1700000000,
      }
    ];

    final items = parseInboxItems(raw);
    expect(items.length, 1);
    final item = items.first;
    expect(item.id, 1926);
    expect(item.jobId, 619);
    expect(item.malId, 235);
    expect(item.showTitle, 'Detective Conan');
    expect(item.type, QuestionType.seasonMap);
    expect(item.title, 'Map Community Season 33');
    expect(
      item.details,
      'Release specifies Season 33, which is not mapped',
    );
    expect(item.choices.length, 3);
    expect(item.choices.first.key, '1050-1093');
    expect(
      item.choices.first.label,
      'Map Season 33 to episodes 1050–1093',
    );
  });

  testWidgets('InboxDynamicCard renders seasonMap question and choice button', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final fakeBridge = FakeCoreBridge(mode: FakeMode.empty);
    final state = StateProvider(bridge: fakeBridge, startPolling: false);

    final item = InboxItem(
      id: 1926,
      jobId: 619,
      malId: 235,
      showTitle: 'Detective Conan',
      title: 'Map Community Season 33',
      details: 'Release specifies Season 33, which is not mapped',
      type: QuestionType.seasonMap,
      choices: [
        InboxChoice(
          key: '1050-1093',
          label: 'Map Season 33 to episodes 1050–1093',
        ),
        InboxChoice(key: 'none_of_these', label: 'None of these'),
        InboxChoice(key: 'search_again', label: 'Search again'),
      ],
      createdAt: DateTime(2026, 11, 9),
    );

    String? resolvedId;
    String? resolvedSummary;

    await tester.pumpWidget(
      MaterialApp(
        theme: ObsidianTheme.darkTheme,
        home: Scaffold(
          body: InboxDynamicCard(
            item: item,
            state: state,
            onResolve: (id, summary) {
              resolvedId = id;
              resolvedSummary = summary;
            },
          ),
        ),
      ),
    );
    await tester.pump();

    expect(find.text('COMMUNITY SEASON MAPPING'), findsOneWidget);
    expect(find.text('Map Community Season 33'), findsOneWidget);
    expect(
      find.text('Map Season 33 to episodes 1050–1093'),
      findsOneWidget,
    );

    await tester.tap(find.text('Map Season 33 to episodes 1050–1093'));
    await tester.pump();

    expect(resolvedId, 'item-1926');
    expect(
      resolvedSummary,
      'Map Season 33 to episodes 1050–1093 for Map Community Season 33',
    );
  });
}
