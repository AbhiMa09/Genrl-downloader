import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'pages/activity_page.dart';
import 'pages/dashboard_page.dart';
import 'pages/inbox_page.dart';
import 'pages/library_page.dart';
import 'pages/queue_page.dart';
import 'pages/rules_page.dart';
import 'pages/search_page.dart';
import 'pages/settings_page.dart';
import 'pages/show_detail_page.dart';
import 'pages/ingestion_stage_page.dart';
import 'services/state_provider.dart';
import 'theme/colors.dart';
import 'theme/theme.dart';
import 'widgets/app_header.dart';
import 'widgets/nav_rail.dart';
import 'widgets/status_strip.dart';

import 'pages/wizard_page.dart';
import 'widgets/tray_notifications_overlay.dart';
import 'widgets/cyber_logo.dart';

void main(List<String> args) {
  WidgetsFlutterBinding.ensureInitialized();
  int initialPage = 0;
  int? initialShowId;
  bool isWizard = false;
  bool isTray = false;
  bool isLogo = false;
  bool isIngestion = false;

  for (int i = 0; i < args.length; i++) {
    if (args[i] == '--page' && i + 1 < args.length) {
      initialPage = int.tryParse(args[i + 1]) ?? 0;
    } else if (args[i] == '--show' && i + 1 < args.length) {
      initialShowId = int.tryParse(args[i + 1]);
    } else if (args[i] == '--wizard') {
      isWizard = true;
    } else if (args[i] == '--tray') {
      isTray = true;
    } else if (args[i] == '--logo') {
      isLogo = true;
    } else if (args[i] == '--ingestion') {
      isIngestion = true;
    }
  }

  runApp(GenrlApp(
    initialPage: initialPage,
    initialShowId: initialShowId,
    isWizard: isWizard,
    isTray: isTray,
    isLogo: isLogo,
    isIngestion: isIngestion,
  ));
}

class GenrlApp extends StatelessWidget {
  final StateProvider? stateProvider;
  final int initialPage;
  final int? initialShowId;
  final bool isWizard;
  final bool isTray;
  final bool isLogo;
  final bool isIngestion;

  const GenrlApp({
    super.key,
    this.stateProvider,
    this.initialPage = 0,
    this.initialShowId,
    this.isWizard = false,
    this.isTray = false,
    this.isLogo = false,
    this.isIngestion = false,
  });

  @override
  Widget build(BuildContext context) {
    Widget homeWidget;
    if (isWizard) {
      homeWidget = WizardPage(onComplete: () {});
    } else if (isTray) {
      homeWidget = Scaffold(
        backgroundColor: ObsidianColors.surface,
        body: Center(child: SystemTrayNotificationOverlay(onClose: () {})),
      );
    } else if (isLogo) {
      homeWidget = const Scaffold(
        backgroundColor: ObsidianColors.surface,
        body: Center(child: CyberLogo(size: 256)),
      );
    } else if (isIngestion) {
      homeWidget = const Scaffold(
        backgroundColor: ObsidianColors.surface,
        body: IngestionStagePage(),
      );
    } else {
      homeWidget = const MainShell();
    }

    return MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) {
          final sp = stateProvider ?? StateProvider();
          if (initialPage > 0) {
            sp.selectPage(initialPage);
          }
          if (initialShowId != null) {
            sp.loadShowDetail(initialShowId!);
          }
          return sp;
        }),
      ],
      child: MaterialApp(
        title: 'Genrl Downloader',
        debugShowCheckedModeBanner: false,
        theme: ObsidianTheme.darkTheme,
        home: homeWidget,
      ),
    );
  }
}

class MainShell extends StatelessWidget {
  const MainShell({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    final isDetailOpen = state.selectedShow != null;

    final pages = <Widget>[
      const DashboardPage(),
      const InboxPage(),
      const QueuePage(),
      const LibraryPage(),
      const SearchPage(),
      const ActivityPage(),
      const RulesPage(),
      const SettingsPage(),
      const IngestionStagePage(),
    ];

    final titles = <String>[
      'Dashboard Overview',
      'Action Inbox',
      'Download Queue',
      'Anime Library',
      'Release Search',
      'Activity Telemetry',
      'Matching Rules',
      'System Settings',
      'Ingestion & Swarm Stage',
    ];

    final activeIndex = state.selectedPageIndex.clamp(0, pages.length - 1);
    final currentPage = isDetailOpen
        ? ShowDetailPage(show: state.selectedShow!)
        : pages[activeIndex];
    final currentTitle = isDetailOpen
        ? 'Anime Detail: ${state.selectedShow!.title}'
        : titles[activeIndex];

    return Scaffold(
      body: Row(
        children: [
          // Left Navigation Rail
          const NavRail(),

          // Main Area
          Expanded(
            child: Column(
              children: [
                // Top Header Bar
                AppHeader(pageTitle: currentTitle),

                // Main Page Content
                Expanded(
                  child: currentPage,
                ),

                // Bottom Status Strip
                const StatusStrip(),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
