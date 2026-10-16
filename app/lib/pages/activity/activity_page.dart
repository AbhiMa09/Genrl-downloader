import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../models/activity_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'activity_filter_dock.dart';
import 'activity_header_context.dart';
import 'activity_telemetry_grid.dart';
import 'activity_timeline_section.dart';

class ActivityPage extends StatefulWidget {
  const ActivityPage({super.key});

  @override
  State<ActivityPage> createState() => _ActivityPageState();
}

class _ActivityPageState extends State<ActivityPage> {
  String _selectedLevel = 'ALL'; // ALL, INFO, COMMITTED, WARN, ERROR
  String _searchFilter = '';
  String _selectedShow = 'All Monitored Shows';
  String _selectedTimeRange = 'Last 7 Days';

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    // Synthetic metrics & calculations based on actual state
    final totalEvents = state.events.length;
    final infoCount = state.events
        .where((e) => e.level == ActivityLevel.info)
        .length;
    final warnCount = state.events
        .where((e) => e.level == ActivityLevel.warn)
        .length;
    final errorCount = state.events
        .where((e) => e.level == ActivityLevel.error)
        .length;
    // We treat events mentioning committed/hardlinked as committed
    final committedCount = state.events
        .where(
          (e) =>
              e.message.toLowerCase().contains('hardlink') ||
              e.message.toLowerCase().contains('committed') ||
              e.message.toLowerCase().contains('completed') ||
              e.message.toLowerCase().contains('moved'),
        )
        .length;

    // Filter events
    final filteredEvents = state.events.where((e) {
      if (_selectedLevel == 'INFO' && e.level != ActivityLevel.info) {
        return false;
      }
      if (_selectedLevel == 'WARN' && e.level != ActivityLevel.warn) {
        return false;
      }
      if (_selectedLevel == 'ERROR' && e.level != ActivityLevel.error) {
        return false;
      }
      if (_selectedLevel == 'COMMITTED') {
        final msg = e.message.toLowerCase();
        if (!msg.contains('hardlink') &&
            !msg.contains('committed') &&
            !msg.contains('completed') &&
            !msg.contains('moved')) {
          return false;
        }
      }
      if (_searchFilter.isNotEmpty) {
        if (!e.message.toLowerCase().contains(_searchFilter.toLowerCase())) {
          return false;
        }
      }
      return true;
    }).toList();

    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 20),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 1400),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // Header Context Strip & Quick Metrics
              ActivityHeaderContext(state: state),
              const SizedBox(height: 20),

              // Filter Control Dock
              ActivityFilterDock(
                selectedLevel: _selectedLevel,
                onSelectLevel: (lvl) => setState(() => _selectedLevel = lvl),
                onSearchChanged: (val) => setState(() => _searchFilter = val),
                selectedShow: _selectedShow,
                onSelectShow: (show) => setState(() => _selectedShow = show),
                selectedTimeRange: _selectedTimeRange,
                onSelectTimeRange: (range) =>
                    setState(() => _selectedTimeRange = range),
                totalCount: totalEvents,
                infoCount: infoCount,
                committedCount: committedCount,
                warnCount: warnCount,
                errorCount: errorCount,
              ),
              const SizedBox(height: 20),

              // Telemetry Activity Breakdown / Bar Sparkline (4 luminous bento cards)
              ActivityTelemetryGrid(state: state),
              const SizedBox(height: 24),

              // Timeline Streams
              ActivityTimelineSection(events: filteredEvents, state: state),
              const SizedBox(height: 24),

              // Pagination & Stream footer
              _buildPaginationFooter(filteredEvents.length, totalEvents),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildPaginationFooter(int showingCount, int totalCount) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(
            'Showing 1 - $showingCount of $totalCount logged events',
            style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
          ),
          Row(
            children: [
              const IconButton(
                onPressed: null,
                icon: Icon(
                  Icons.chevron_left_rounded,
                  color: ObsidianColors.outline,
                ),
                iconSize: 20,
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  '1 / 1',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.primary,
                  ),
                ),
              ),
              const IconButton(
                onPressed: null,
                icon: Icon(
                  Icons.chevron_right_rounded,
                  color: ObsidianColors.outline,
                ),
                iconSize: 20,
              ),
            ],
          ),
        ],
      ),
    );
  }
}
