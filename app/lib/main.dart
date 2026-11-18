import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';
import 'package:provider/provider.dart';

import 'pages/activity_page.dart';
import 'pages/dashboard_page.dart';
import 'pages/dramas_page.dart';
import 'pages/inbox_page.dart';
import 'pages/library_page.dart';
import 'pages/movies_page.dart';
import 'pages/queue_page.dart';
import 'pages/rules_page.dart';
import 'pages/search_page.dart';
import 'pages/settings_page.dart';
import 'pages/show_detail_page.dart';
import 'pages/wizard/guided_first_run_page.dart';
import 'pages/wizard_page.dart';
import 'services/core_bridge.dart';
import 'services/state_provider.dart';
import 'state/dramas_cubit.dart';
import 'state/indexer_cubit.dart';
import 'state/movies_cubit.dart';
import 'theme/colors.dart';
import 'theme/theme.dart';
import 'theme/typography.dart';
import 'widgets/app_header.dart';
import 'widgets/cyber_logo.dart';
import 'widgets/loading/loading.dart';
import 'widgets/nav_rail.dart';
import 'widgets/status_strip.dart';
import 'widgets/tray_notifications_overlay.dart';

void main(List<String> args) {
  WidgetsFlutterBinding.ensureInitialized();
  GoogleFonts.config.allowRuntimeFetching = false;
  int initialPage = 0;
  int? initialShowId;
  bool isWizard = false;
  bool isTray = false;
  bool isLogo = false;
  bool isGallery = false;

  String? dataDir;
  for (int i = 0; i < args.length; i++) {
    if (args[i].startsWith('--data-dir=')) {
      dataDir = args[i].substring('--data-dir='.length);
    } else if (args[i] == '--data-dir' && i + 1 < args.length) {
      dataDir = args[i + 1];
    } else if (args[i].startsWith('--page=')) {
      final name = args[i].substring('--page='.length).toLowerCase();
      initialPage = _parsePageIndex(name);
    } else if (args[i] == '--page' && i + 1 < args.length) {
      final name = args[i + 1].toLowerCase();
      initialPage = _parsePageIndex(name);
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

  runApp(
    GenrlApp(
      dataDir: dataDir,
      initialPage: initialPage,
      initialShowId: initialShowId,
      isWizard: isWizard,
      isTray: isTray,
      isLogo: isLogo,
      isGallery: isGallery,
    ),
  );
}

int _parsePageIndex(String value) {
  switch (value.trim().toLowerCase()) {
    case 'dashboard':
      return 0;
    case 'inbox':
      return 1;
    case 'queue':
      return 2;
    case 'library':
      return 3;
    case 'search':
      return 4;
    case 'activity':
      return 5;
    case 'rules':
      return 6;
    case 'settings':
      return 7;
    case 'movies':
      return 8;
    case 'dramas':
      return 9;
    default:
      return int.tryParse(value) ?? 0;
  }
}

class GenrlApp extends StatefulWidget {
  final StateProvider? stateProvider;
  final String? dataDir;
  final int initialPage;
  final int? initialShowId;
  final bool isWizard;
  final bool isTray;
  final bool isLogo;
  final bool isGallery;

  const GenrlApp({
    super.key,
    this.stateProvider,
    this.dataDir,
    this.initialPage = 0,
    this.initialShowId,
    this.isWizard = false,
    this.isTray = false,
    this.isLogo = false,
    this.isGallery = false,
  });

  @override
  State<GenrlApp> createState() => _GenrlAppState();
}

class _GenrlAppState extends State<GenrlApp> {
  late bool _showWizard;

  @override
  void initState() {
    super.initState();
    _showWizard = widget.isWizard;
  }

  @override
  Widget build(BuildContext context) {
    return MultiProvider(
      providers: [
        ChangeNotifierProvider<StateProvider>(
          create: (_) {
            final sp =
                widget.stateProvider ??
                StateProvider(
                  bridge: CoreBridge(customDataDir: widget.dataDir),
                );
            if (widget.initialPage > 0) {
              sp.selectPage(widget.initialPage);
            }
            if (widget.initialShowId != null) {
              // The show page lives under Library (page 3).
              if (widget.initialPage == 0) sp.selectPage(3);
              sp.loadShowDetail(widget.initialShowId!);
            }
            return sp;
          },
        ),
        ChangeNotifierProxyProvider<StateProvider, MoviesCubit>(
          create: (ctx) => ctx.read<StateProvider>().moviesCubit,
          update: (_, sp, prev) => sp.moviesCubit,
        ),
        ChangeNotifierProxyProvider<StateProvider, DramasCubit>(
          create: (ctx) => ctx.read<StateProvider>().dramasCubit,
          update: (_, sp, prev) => sp.dramasCubit,
        ),
        ChangeNotifierProxyProvider<StateProvider, IndexerCubit>(
          create: (ctx) => ctx.read<StateProvider>().indexerCubit,
          update: (_, sp, prev) => sp.indexerCubit,
        ),
      ],
      child: MaterialApp(
        title: 'Genrl Downloader',
        debugShowCheckedModeBanner: false,
        theme: ObsidianTheme.darkTheme,
        routes: {
          '/dashboard': (ctx) => const MainShell(),
          '/inbox': (ctx) => const MainShell(),
          '/queue': (ctx) => const MainShell(),
          '/library': (ctx) => const MainShell(),
          '/search': (ctx) => const MainShell(),
          '/activity': (ctx) => const MainShell(),
          '/rules': (ctx) => const MainShell(),
          '/settings': (ctx) => const MainShell(),
          '/movies': (ctx) => const MainShell(),
          '/dramas': (ctx) => const MainShell(),
          if (kDebugMode)
            '/debug/loading-gallery': (ctx) => const LoadingDebugGallery(),
        },
        home: Consumer<StateProvider>(
          builder: (context, sp, _) {
            if (widget.isGallery && kDebugMode) {
              return const LoadingDebugGallery();
            }
            if (_showWizard || !sp.firstRunCompleted) {
              if (Platform.isWindows) {
                return GuidedFirstRunPage(
                  onComplete: () {
                    setState(() => _showWizard = false);
                    sp.setFirstRunCompleted(true);
                  },
                );
              }
              return WizardPage(
                onComplete: () {
                  setState(() => _showWizard = false);
                  sp.setFirstRunCompleted(true);
                },
              );
            }
            if (widget.isTray) {
              return Scaffold(
                backgroundColor: ObsidianColors.surface,
                body: Center(
                  child: SystemTrayNotificationOverlay(onClose: () {}),
                ),
              );
            }
            if (widget.isLogo) {
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
      const MoviesPage(),
      const DramasPage(),
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
      'Movies',
      'Dramas',
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

                if (state.bridge.customDataDir != null)
                  Container(
                    width: double.infinity,
                    padding: const EdgeInsets.symmetric(
                      horizontal: 16,
                      vertical: 6,
                    ),
                    color: ObsidianColors.tertiary.withOpacity(0.15),
                    child: Row(
                      children: [
                        const Icon(
                          Icons.science_outlined,
                          size: 16,
                          color: ObsidianColors.tertiary,
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            'SCRATCH DB: ${state.bridge.customDataDir}',
                            style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.tertiary,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ],
                    ),
                  ),

                // Connection status banner
                const ConnectionBanner(),

                // Main Page Content
                Expanded(child: currentPage),

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
