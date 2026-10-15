import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'pages/activity_page.dart';
import 'pages/dashboard_page.dart';
import 'pages/inbox_page.dart';
import 'pages/library_page.dart';
import 'pages/loading_gallery_page.dart';
import 'pages/queue_page.dart';
import 'pages/rules_page.dart';
import 'pages/search_page.dart';
import 'pages/settings_page.dart';
import 'pages/show_detail_page.dart';
import 'pages/wizard_page.dart';
import 'services/state_provider.dart';
import 'theme/colors.dart';
import 'theme/theme.dart';
import 'widgets/app_header.dart';
import 'widgets/cyber_logo.dart';
import 'widgets/loading/connection_banner.dart';
import 'widgets/nav_rail.dart';
import 'widgets/status_strip.dart';
import 'widgets/tray_notifications_overlay.dart';

void main(List<String> args) {
  WidgetsFlutterBinding.ensureInitialized();
  int initialPage = 0;
  int? initialShowId;
  bool isWizard = false;
  bool isTray = false;
  bool isLogo = false;
  bool isGallery = false;

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
    } else if (args[i] == '--gallery' || args[i] == '--debug-gallery') {
      isGallery = true;
    }
  }

  runApp(GenrlApp(
    initialPage: initialPage,
    initialShowId: initialShowId,
    isWizard: isWizard,
    isTray: isTray,
    isLogo: isLogo,
    isGallery: isGallery,
  ));
}

class GenrlApp extends StatelessWidget {
  final StateProvider? stateProvider;
  final int initialPage;
  final int? initialShowId;
  final bool isWizard;
  final bool isTray;
  final bool isLogo;
  final bool isGallery;

  const GenrlApp({
    super.key,
    this.stateProvider,
    this.initialPage = 0,
    this.initialShowId,
    this.isWizard = false,
    this.isTray = false,
    this.isLogo = false,
    this.isGallery = false,
  });

  @override
  Widget build(BuildContext context) {
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
        routes: {
          if (kDebugMode) '/debug/loading-gallery': (ctx) => const LoadingGalleryPage(),
        },
        home: Consumer<StateProvider>(
          builder: (context, sp, _) {
            if (isGallery && kDebugMode) {
              return const LoadingGalleryPage();
            }
            if (isWizard || !sp.firstRunCompleted) {
              return WizardPage(
                onComplete: () {
                  sp.setFirstRunCompleted(true);
                },
              );
            }
            if (isTray) {
              return Scaffold(
                backgroundColor: ObsidianColors.surface,
                body: Center(child: SystemTrayNotificationOverlay(onClose: () {})),
              );
            }
            if (isLogo) {
              return const Scaffold(
                backgroundColor: ObsidianColors.surface,
                body: Center(child: CyberLogo(size: 256)),
              );
            }
            return const MainShell();
          },
        ),
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

                // Connection status banner
                const ConnectionBanner(),

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
