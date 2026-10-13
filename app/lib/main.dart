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
import 'theme/theme.dart';
import 'widgets/app_header.dart';
import 'widgets/nav_rail.dart';
import 'widgets/status_strip.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const GenrlApp());
}

class GenrlApp extends StatelessWidget {
  final StateProvider? stateProvider;
  const GenrlApp({super.key, this.stateProvider});

  @override
  Widget build(BuildContext context) {
    return MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) => stateProvider ?? StateProvider()),
      ],
      child: MaterialApp(
        title: 'Genrl Downloader',
        debugShowCheckedModeBanner: false,
        theme: ObsidianTheme.darkTheme,
        home: const MainShell(),
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
