import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provider/provider.dart';

import 'package:genrl_downloader/pages/wizard/guided_first_run_help_dialog.dart';
import 'package:genrl_downloader/pages/wizard/guided_first_run_page.dart';
import 'package:genrl_downloader/pages/wizard/guided_folders_screen.dart';
import 'package:genrl_downloader/pages/wizard/guided_mal_screen.dart';
import 'package:genrl_downloader/pages/wizard/guided_qbittorrent_screen.dart';
import 'package:genrl_downloader/pages/wizard/guided_welcome_screen.dart';
import 'package:genrl_downloader/services/state_provider.dart';
import 'package:genrl_downloader/theme/theme.dart';

import 'fake_core_bridge.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  late FakeCoreBridge fakeBridge;
  late StateProvider state;

  setUp(() {
    fakeBridge = FakeCoreBridge(mode: FakeMode.data);
    state = StateProvider(bridge: fakeBridge, startPolling: false);
  });

  Widget createTestApp({required Widget home}) {
    return ChangeNotifierProvider<StateProvider>.value(
      value: state,
      child: MaterialApp(
        theme: ObsidianTheme.darkTheme,
        home: Scaffold(body: SingleChildScrollView(child: home)),
      ),
    );
  }

  Widget createTestPage({required Widget page}) {
    return ChangeNotifierProvider<StateProvider>.value(
      value: state,
      child: MaterialApp(theme: ObsidianTheme.darkTheme, home: page),
    );
  }

  testWidgets(
    'Step 1 Welcome Screen renders what Genrl does, prerequisites, and Why line',
    (tester) async {
      tester.view.physicalSize = const Size(1600, 1200);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        createTestApp(home: GuidedWelcomeScreen(onNext: () {})),
      );
      await tester.pumpAndSettle();

      expect(find.text('Welcome to Genrl Downloader'), findsOneWidget);
      expect(find.textContaining('What Genrl Does:'), findsOneWidget);
      expect(find.text('MyAnimeList Account'), findsOneWidget);
      expect(find.text('qBittorrent (Installed & Running)'), findsOneWidget);
      expect(
        find.text('Free Disk Space (Recommended >= 10 GB)'),
        findsOneWidget,
      );
      expect(find.textContaining('Why do we ask this?'), findsOneWidget);
    },
  );

  testWidgets(
    'Step 2 qBittorrent Screen renders instructions, download button, and Check again',
    (tester) async {
      tester.view.physicalSize = const Size(1600, 1200);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      final urlController = TextEditingController(
        text: 'http://127.0.0.1:8080',
      );
      final userController = TextEditingController(text: 'admin');
      final passwordController = TextEditingController();

      await tester.pumpWidget(
        createTestApp(
          home: GuidedQbittorrentScreen(
            urlController: urlController,
            userController: userController,
            passwordController: passwordController,
            state: state,
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Connect qBittorrent'), findsOneWidget);
      expect(find.text('Check again'), findsOneWidget);
      expect(
        find.textContaining('Numbered Setup Instructions:'),
        findsOneWidget,
      );
      expect(find.textContaining('Tools -> Options'), findsOneWidget);
      expect(
        find.textContaining('Bypass authentication for clients on localhost'),
        findsOneWidget,
      );
      expect(find.textContaining('Why do we ask this?'), findsOneWidget);

      await tester.tap(find.text('Check again'));
      await tester.pumpAndSettle();
    },
  );

  testWidgets(
    'Step 3 MAL Screen renders single Connect MAL button, explanation, and guest mode',
    (tester) async {
      tester.view.physicalSize = const Size(1600, 1200);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      final userController = TextEditingController(text: 'akumasan2');
      final clientIdController = TextEditingController();

      await tester.pumpWidget(
        createTestApp(
          home: GuidedMalScreen(
            userController: userController,
            clientIdController: clientIdController,
            state: state,
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Connect MyAnimeList'), findsWidgets);
      expect(find.text('Check again'), findsOneWidget);
      expect(find.textContaining('What will happen:'), findsOneWidget);
      expect(find.textContaining('Your web browser will open'), findsOneWidget);
      expect(find.textContaining('Or use Guest Mode'), findsOneWidget);
      expect(find.textContaining('Why do we ask this?'), findsOneWidget);

      await tester.tap(find.textContaining('Or use Guest Mode'));
      await tester.pumpAndSettle();
      expect(find.text('Public MAL Username'), findsOneWidget);
    },
  );

  testWidgets(
    'Step 4 Folders Screen renders cross-drive warning, free space, and NTFS note',
    (tester) async {
      tester.view.physicalSize = const Size(1600, 1200);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      final libController = TextEditingController(
        text: r'C:\Users\User\Videos\Anime',
      );
      final dlController = TextEditingController(
        text: r'D:\Users\User\Downloads\Genrl',
      );

      await tester.pumpWidget(
        createTestApp(
          home: GuidedFoldersScreen(
            libraryController: libController,
            downloadController: dlController,
            state: state,
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Storage & Library Folders'), findsOneWidget);
      expect(find.text('Check again'), findsOneWidget);
      expect(
        find.textContaining('Folders are on different drives (C: vs D:)'),
        findsOneWidget,
      );
      expect(find.textContaining('NTFS note:'), findsOneWidget);
      expect(find.textContaining('Why do we ask this?'), findsOneWidget);

      dlController.text = r'C:\Users\User\Downloads\Genrl';
      await tester.pumpAndSettle();
      expect(find.textContaining('Same drive detected'), findsOneWidget);
    },
  );

  testWidgets(
    'Help Panel displays the 5 most common troubleshooting problems',
    (tester) async {
      tester.view.physicalSize = const Size(1600, 1200);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(() {
        tester.view.resetPhysicalSize();
        tester.view.resetDevicePixelRatio();
      });

      await tester.pumpWidget(
        createTestApp(home: const GuidedFirstRunHelpDialog()),
      );
      await tester.pumpAndSettle();

      expect(find.text('Help & Troubleshooting'), findsOneWidget);
      expect(find.textContaining('SmartScreen'), findsWidgets);
      expect(find.textContaining('Antivirus'), findsWidgets);
      expect(find.textContaining('qBittorrent Not Detected'), findsWidgets);
      expect(find.textContaining('MyAnimeList Login Window'), findsWidgets);

      final fifthFinder = find.text('No Free Disk Space / Storage Warnings');
      await tester.scrollUntilVisible(
        fifthFinder,
        100,
        scrollable: find.byType(Scrollable).last,
      );
      expect(fifthFinder, findsOneWidget);
    },
  );

  testWidgets('GuidedFirstRunPage navigation through steps and launch strip', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1600, 1200);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() {
      tester.view.resetPhysicalSize();
      tester.view.resetDevicePixelRatio();
    });

    await tester.pumpWidget(
      createTestPage(
        page: GuidedFirstRunPage(initialStep: 0, onComplete: () {}),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Genrl Setup Guide'), findsOneWidget);
    expect(find.text('1. Welcome'), findsOneWidget);

    // Advance to step 2 (qBit)
    final contBtn = find.text('Continue');
    await tester.ensureVisible(contBtn);
    await tester.tap(contBtn);
    await tester.pumpAndSettle();
    expect(find.text('2. qBittorrent'), findsOneWidget);

    // Advance to step 3 (MAL)
    await tester.ensureVisible(contBtn);
    await tester.tap(contBtn);
    await tester.pumpAndSettle();
    expect(find.text('3. MyAnimeList'), findsOneWidget);

    // Advance to step 4 (Folders)
    await tester.ensureVisible(contBtn);
    await tester.tap(contBtn);
    await tester.pumpAndSettle();
    expect(find.text('4. Folders'), findsOneWidget);

    // Advance to step 5 (Review)
    await tester.ensureVisible(contBtn);
    await tester.tap(contBtn);
    await tester.pumpAndSettle();
    expect(find.text('5. Review & Launch'), findsOneWidget);
    expect(find.text('Turn automation on'), findsOneWidget);
    expect(find.text('Start with automation off'), findsOneWidget);
  });
}
