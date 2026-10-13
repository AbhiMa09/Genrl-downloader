import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provider/provider.dart';
import 'package:genrl_downloader/pages/settings_page.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/theme/theme.dart';
import 'fake_core_bridge.dart';

Widget createTestApp(StateProvider stateProvider) {
  return ChangeNotifierProvider<StateProvider>.value(
    value: stateProvider,
    child: MaterialApp(
      theme: ObsidianTheme.darkTheme,
      home: const Scaffold(body: SettingsPage()),
    ),
  );
}

void main() {
  setUp(() {
    TestWidgetsFlutterBinding.ensureInitialized();
  });

  testWidgets('Settings Page renders profile card, MAL connection, and storage locations', (tester) async {
    tester.view.physicalSize = const Size(1280, 1000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final fakeBridge = FakeCoreBridge(mode: FakeMode.data);
    final state = StateProvider(bridge: fakeBridge, startPolling: false);
    await state.refreshAll();

    await tester.pumpWidget(createTestApp(state));
    await tester.pump();

    // Verify Profile Card
    expect(find.text('Application Settings'), findsOneWidget);
    expect(find.text('TestOtaku'), findsOneWidget);
    expect(find.text('Watching: '), findsOneWidget);
    expect(find.text('12'), findsOneWidget);
    expect(find.text('Completed: '), findsOneWidget);
    expect(find.text('140'), findsOneWidget);

    // Verify MAL Account Section & Buttons
    expect(find.text('Disconnect'), findsOneWidget);
    expect(find.text('Sync Now'), findsOneWidget);

    // Verify Storage Locations
    expect(find.text('Same Filesystem (Hardlink Enabled)'), findsOneWidget);
  });

  testWidgets('Settings Page renders not connected state when unlinked', (tester) async {
    tester.view.physicalSize = const Size(1280, 1000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    final fakeBridge = FakeCoreBridge(mode: FakeMode.empty);
    final state = StateProvider(bridge: fakeBridge, startPolling: false);
    await state.refreshAll();

    await tester.pumpWidget(createTestApp(state));
    await tester.pump();

    // Verify Not Connected status
    expect(find.text('Not Connected'), findsWidgets);
    expect(find.text('Connect via Browser (PKCE)'), findsOneWidget);
    expect(find.text('Save Public Username'), findsOneWidget);
  });
}
