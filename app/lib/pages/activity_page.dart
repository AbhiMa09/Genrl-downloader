import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/activity_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';

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
    final infoCount = state.events.where((e) => e.level == ActivityLevel.info).length;
    final warnCount = state.events.where((e) => e.level == ActivityLevel.warn).length;
    final errorCount = state.events.where((e) => e.level == ActivityLevel.error).length;
    // We treat events mentioning committed/hardlinked as committed
    final committedCount = state.events.where((e) =>
      e.message.toLowerCase().contains('hardlink') ||
      e.message.toLowerCase().contains('committed') ||
      e.message.toLowerCase().contains('completed') ||
      e.message.toLowerCase().contains('moved')
    ).length;

    // Filter events
    final filteredEvents = state.events.where((e) {
      if (_selectedLevel == 'INFO' && e.level != ActivityLevel.info) return false;
      if (_selectedLevel == 'WARN' && e.level != ActivityLevel.warn) return false;
      if (_selectedLevel == 'ERROR' && e.level != ActivityLevel.error) return false;
      if (_selectedLevel == 'COMMITTED') {
        final msg = e.message.toLowerCase();
        if (!msg.contains('hardlink') && !msg.contains('committed') && !msg.contains('completed') && !msg.contains('moved')) {
          return false;
        }
      }
      if (_searchFilter.isNotEmpty) {
        if (!e.message.toLowerCase().contains(_searchFilter.toLowerCase())) return false;
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
              _buildHeaderContext(context, state),
              const SizedBox(height: 20),

              // Filter Control Dock
              _buildFilterControlDock(
                totalCount: totalEvents,
                infoCount: infoCount,
                committedCount: committedCount,
                warnCount: warnCount,
                errorCount: errorCount,
              ),
              const SizedBox(height: 20),

              // Telemetry Activity Breakdown / Bar Sparkline (4 luminous bento cards)
              _buildTelemetryGrid(state),
              const SizedBox(height: 24),

              // Timeline Streams
              _buildTimelineSection(context, filteredEvents, state),
              const SizedBox(height: 24),

              // Pagination & Stream footer
              _buildPaginationFooter(filteredEvents.length, totalEvents),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildHeaderContext(BuildContext context, StateProvider state) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                spacing: 10,
                runSpacing: 4,
                children: [
                  Text('Activity & Decision Log', style: ObsidianTypography.headlineLg()),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                    decoration: BoxDecoration(
                      color: ObsidianColors.primary.withOpacity(0.12),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      'AUDIT STREAM',
                      style: ObsidianTypography.monoCode(color: ObsidianColors.primary).copyWith(fontWeight: FontWeight.bold),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 4),
              Text(
                'Real-time pipeline decisions, scoring metrics, indexer queries, and hardlink telemetry.',
                style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
              ),
            ],
          ),
        ),
        Row(
          children: [
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLow,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: ObsidianColors.border),
              ),
              child: Row(
                children: [
                  Container(
                    width: 7,
                    height: 7,
                    decoration: const BoxDecoration(
                      shape: BoxShape.circle,
                      color: ObsidianColors.tertiary,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text('Auto-tail: ACTIVE', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface)),
                  const SizedBox(width: 6),
                  Text('500ms', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                ],
              ),
            ),
            const SizedBox(width: 10),
            ElevatedButton.icon(
              onPressed: () {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(
                    content: Text('Audit log exported to /tmp/genrl-activity.log'),
                    backgroundColor: ObsidianColors.surfaceContainerHigh,
                  ),
                );
              },
              icon: const Icon(Icons.ios_share_rounded, size: 16, color: ObsidianColors.primary),
              label: const Text('Export'),
              style: ElevatedButton.styleFrom(
                backgroundColor: ObsidianColors.surfaceContainer,
                foregroundColor: ObsidianColors.onSurface,
                elevation: 0,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(8),
                  side: const BorderSide(color: ObsidianColors.border),
                ),
              ),
            ),
            const SizedBox(width: 8),
            OutlinedButton.icon(
              onPressed: () {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(
                    content: Text('Logs older than 30 days purged'),
                    backgroundColor: ObsidianColors.surfaceContainerHigh,
                  ),
                );
              },
              icon: const Icon(Icons.delete_sweep_rounded, size: 16, color: ObsidianColors.outline),
              label: const Text('Purge > 30d'),
              style: OutlinedButton.styleFrom(
                side: const BorderSide(color: ObsidianColors.border),
                foregroundColor: ObsidianColors.onSurfaceVariant,
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildFilterControlDock({
    required int totalCount,
    required int infoCount,
    required int committedCount,
    required int warnCount,
    required int errorCount,
  }) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        children: [
          Row(
            children: [
              // Category chips
              _buildCategoryChip('ALL', totalCount, ObsidianColors.primary, null),
              const SizedBox(width: 8),
              _buildCategoryChip('INFO', infoCount, ObsidianColors.tertiary, ObsidianColors.tertiary),
              const SizedBox(width: 8),
              _buildCategoryChip('COMMITTED', committedCount, ObsidianColors.secondary, ObsidianColors.secondary),
              const SizedBox(width: 8),
              _buildCategoryChip('WARN', warnCount, ObsidianColors.warning, ObsidianColors.warning),
              const SizedBox(width: 8),
              _buildCategoryChip('ERROR', errorCount, ObsidianColors.error, ObsidianColors.error),

              const SizedBox(width: 16),
              // Search input
              Expanded(
                child: Container(
                  height: 36,
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLowest,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: ObsidianColors.border),
                  ),
                  child: Row(
                    children: [
                      const Icon(Icons.filter_alt_rounded, size: 16, color: ObsidianColors.outline),
                      const SizedBox(width: 8),
                      Expanded(
                        child: TextField(
                          onChanged: (val) => setState(() => _searchFilter = val),
                          style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
                          decoration: InputDecoration(
                            hintText: 'Filter message, hash, rule ID, indexer...',
                            hintStyle: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                            border: InputBorder.none,
                            isDense: true,
                            contentPadding: EdgeInsets.zero,
                          ),
                        ),
                      ),
                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1.5),
                        decoration: BoxDecoration(
                          color: ObsidianColors.surfaceContainerHighest,
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text('Ctrl+F', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 10),

              // Series Scope Dropdown
              Container(
                height: 36,
                padding: const EdgeInsets.symmetric(horizontal: 10),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLowest,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: DropdownButtonHideUnderline(
                  child: Builder(builder: (context) {
                    final state = context.watch<StateProvider>();
                    final showTitles = state.shows.map((s) => s.title).toList();
                    final currentVal = (showTitles.contains(_selectedShow) || _selectedShow == 'All Monitored Shows')
                        ? _selectedShow
                        : 'All Monitored Shows';
                    return DropdownButton<String>(
                      value: currentVal,
                      dropdownColor: ObsidianColors.surfaceContainerHigh,
                      icon: const Icon(Icons.expand_more_rounded, size: 16, color: ObsidianColors.outline),
                      style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
                      items: [
                        const DropdownMenuItem(value: 'All Monitored Shows', child: Text('All Monitored Shows')),
                        ...showTitles.map((t) => DropdownMenuItem(value: t, child: Text(t))),
                      ],
                      onChanged: (val) {
                        if (val != null) setState(() => _selectedShow = val);
                      },
                    );
                  }),
                ),
              ),
              const SizedBox(width: 8),

              // Time Range Dropdown
              Container(
                height: 36,
                padding: const EdgeInsets.symmetric(horizontal: 10),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLowest,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: DropdownButtonHideUnderline(
                  child: DropdownButton<String>(
                    value: _selectedTimeRange,
                    dropdownColor: ObsidianColors.surfaceContainerHigh,
                    icon: const Icon(Icons.schedule_rounded, size: 16, color: ObsidianColors.outline),
                    style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
                    items: const [
                      DropdownMenuItem(value: 'Last 7 Days', child: Text('Last 7 Days')),
                      DropdownMenuItem(value: 'Today', child: Text('Today')),
                      DropdownMenuItem(value: 'Yesterday', child: Text('Yesterday')),
                      DropdownMenuItem(value: 'Last 30 Days', child: Text('Last 30 Days')),
                    ],
                    onChanged: (val) {
                      if (val != null) setState(() => _selectedTimeRange = val);
                    },
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildCategoryChip(String label, int count, Color color, Color? dotColor) {
    final isSelected = _selectedLevel == label;
    return InkWell(
      onTap: () => setState(() => _selectedLevel = label),
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: isSelected
              ? (label == 'ALL' ? ObsidianColors.primaryContainer : color.withOpacity(0.18))
              : ObsidianColors.surfaceContainer,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isSelected ? color.withOpacity(0.5) : ObsidianColors.border,
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (dotColor != null) ...[
              Container(
                width: 6,
                height: 6,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: dotColor,
                ),
              ),
              const SizedBox(width: 6),
            ],
            Text(
              label,
              style: ObsidianTypography.monoCode(
                color: isSelected
                    ? (label == 'ALL' ? ObsidianColors.onPrimaryContainer : color)
                    : ObsidianColors.onSurfaceVariant,
              ).copyWith(fontWeight: FontWeight.bold),
            ),
            const SizedBox(width: 6),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLowest.withOpacity(0.6),
                borderRadius: BorderRadius.circular(10),
              ),
              child: Text(
                '$count',
                style: ObsidianTypography.labelBadge(
                  color: isSelected
                      ? (label == 'ALL' ? ObsidianColors.onPrimaryContainer : color)
                      : ObsidianColors.outline,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildTelemetryGrid(StateProvider state) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final cardWidth = (constraints.maxWidth - 36) / 4;
        return Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            _buildTelemetryCard(
              title: 'INGESTION RATE',
              icon: Icons.bolt_rounded,
              iconColor: ObsidianColors.tertiary,
              value: '19.4',
              unit: 'events/min',
              progress: 0.65,
              progressColor: ObsidianColors.tertiary,
              width: cardWidth,
            ),
            _buildTelemetryCard(
              title: 'DECISION ACCEPTANCE',
              icon: Icons.rule_folder_rounded,
              iconColor: ObsidianColors.primary,
              value: '94.2%',
              unit: 'auto-commit',
              progress: 0.942,
              progressColor: ObsidianColors.primary,
              width: cardWidth,
            ),
            _buildTelemetryCard(
              title: 'DISCARDED RELEASES',
              icon: Icons.filter_list_off_rounded,
              iconColor: ObsidianColors.secondary,
              value: '148',
              unit: 'filtered',
              progress: 0.38,
              progressColor: ObsidianColors.secondary,
              width: cardWidth,
            ),
            _buildTelemetryCard(
              title: 'ERROR EXCLUSIONS',
              icon: Icons.error_outline_rounded,
              iconColor: ObsidianColors.error,
              value: '${state.events.where((e) => e.level == ActivityLevel.error).length}',
              unit: 'unresolved',
              progress: 0.15,
              progressColor: ObsidianColors.error,
              width: cardWidth,
            ),
          ],
        );
      },
    );
  }

  Widget _buildTelemetryCard({
    required String title,
    required IconData icon,
    required Color iconColor,
    required String value,
    required String unit,
    required double progress,
    required Color progressColor,
    required double width,
  }) {
    return Container(
      width: width,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(title, style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
              Icon(icon, size: 16, color: iconColor),
            ],
          ),
          const SizedBox(height: 6),
          Row(
            crossAxisAlignment: CrossAxisAlignment.baseline,
            textBaseline: TextBaseline.alphabetic,
            children: [
              Text(value, style: ObsidianTypography.headlineMd()),
              const SizedBox(width: 6),
              Text(unit, style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
            ],
          ),
          const SizedBox(height: 8),
          ClipRRect(
            borderRadius: BorderRadius.circular(4),
            child: LinearProgressIndicator(
              value: progress.clamp(0.0, 1.0),
              backgroundColor: ObsidianColors.surfaceContainer,
              valueColor: AlwaysStoppedAnimation<Color>(progressColor),
              minHeight: 4,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildTimelineSection(BuildContext context, List<ActivityItem> events, StateProvider state) {
    if (events.isEmpty) {
      return Container(
        padding: const EdgeInsets.all(32),
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainerLow,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: ObsidianColors.border),
        ),
        child: Center(
          child: Column(
            children: [
              const Icon(Icons.notes_rounded, size: 36, color: ObsidianColors.outline),
              const SizedBox(height: 12),
              Text('No events match the selected criteria', style: ObsidianTypography.bodyMd(color: ObsidianColors.outline)),
            ],
          ),
        ),
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Date Header Banner (Sticky styling)
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainerLowest,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: ObsidianColors.border),
          ),
          child: Row(
            children: [
              const Icon(Icons.calendar_today_rounded, size: 16, color: ObsidianColors.primary),
              const SizedBox(width: 8),
              Text('Today — Recorded Pipeline Events', style: ObsidianTypography.titleSm()),
              const Spacer(),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('${events.length} events logged', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),

        // Timeline items with vertical guide line
        ListView.separated(
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          itemCount: events.length,
          separatorBuilder: (_, __) => const SizedBox(height: 8),
          itemBuilder: (context, index) {
            final event = events[index];
            return _buildTimelineEventCard(context, event, state);
          },
        ),
      ],
    );
  }

  Widget _buildTimelineEventCard(BuildContext context, ActivityItem event, StateProvider state) {
    Color levelColor = ObsidianColors.primary;
    String badgeTag = 'SCORER';
    IconData eventIcon = Icons.rule_rounded;

    if (event.level == ActivityLevel.error) {
      levelColor = ObsidianColors.error;
      badgeTag = 'ERROR';
      eventIcon = Icons.cloud_off_rounded;
    } else if (event.level == ActivityLevel.warn) {
      levelColor = ObsidianColors.warning;
      badgeTag = 'WARN';
      eventIcon = Icons.warning_amber_rounded;
    } else if (event.message.toLowerCase().contains('hardlink') ||
               event.message.toLowerCase().contains('moved') ||
               event.message.toLowerCase().contains('committed')) {
      levelColor = ObsidianColors.secondary;
      badgeTag = 'DISK_IO';
      eventIcon = Icons.folder_open_rounded;
    } else if (event.message.toLowerCase().contains('mal') ||
               event.message.toLowerCase().contains('metadata') ||
               event.message.toLowerCase().contains('sync')) {
      levelColor = ObsidianColors.tertiary;
      badgeTag = 'METADATA';
      eventIcon = Icons.sync_rounded;
    }

    // Try finding matching show poster if available
    String? posterUrl;
    if (event.malId != null) {
      final match = state.trackedShows.where((s) => s.id == event.malId).firstOrNull;
      posterUrl = match?.posterUrl;
    }

    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: event.level == ActivityLevel.error
            ? ObsidianColors.error.withOpacity(0.06)
            : ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: event.level == ActivityLevel.error
              ? ObsidianColors.error.withOpacity(0.3)
              : ObsidianColors.border,
        ),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // 2:3 Cover Poster or Icon
          if (posterUrl != null && posterUrl.isNotEmpty)
            Container(
              width: 40,
              height: 60,
              margin: const EdgeInsets.only(right: 12),
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(4),
                color: ObsidianColors.surfaceContainerHighest,
              ),
              clipBehavior: Clip.antiAlias,
              child: Image.network(
                posterUrl,
                fit: BoxFit.cover,
                errorBuilder: (_, __, ___) => const Icon(Icons.movie_rounded, size: 20, color: ObsidianColors.outline),
              ),
            )
          else
            Container(
              width: 38,
              height: 38,
              margin: const EdgeInsets.only(right: 12),
              decoration: BoxDecoration(
                color: levelColor.withOpacity(0.12),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Icon(eventIcon, size: 20, color: levelColor),
            ),

          // Main text details
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text(event.formattedTime, style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1.5),
                      decoration: BoxDecoration(
                        color: levelColor.withOpacity(0.18),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        badgeTag,
                        style: ObsidianTypography.labelBadge(color: levelColor),
                      ),
                    ),
                    const SizedBox(width: 8),
                    if (event.jobId != null)
                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1.5),
                        decoration: BoxDecoration(
                          color: ObsidianColors.surfaceContainerHighest,
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          'Job #${event.jobId}',
                          style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurfaceVariant),
                        ),
                      ),
                  ],
                ),
                const SizedBox(height: 6),
                Text(
                  event.message,
                  style: ObsidianTypography.bodyMd(color: ObsidianColors.onSurface),
                ),
              ],
            ),
          ),

          // Action button / telemetry
          const SizedBox(width: 12),
          Column(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLowest,
                  borderRadius: BorderRadius.circular(4),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Text(
                  event.level == ActivityLevel.error ? 'ATTENTION' : 'AUDIT #0${event.id}',
                  style: ObsidianTypography.labelBadge(
                    color: event.level == ActivityLevel.error ? ObsidianColors.error : ObsidianColors.outline,
                  ),
                ),
              ),
              const SizedBox(height: 8),
              if (event.level == ActivityLevel.error)
                TextButton.icon(
                  onPressed: () => state.refreshAll(),
                  icon: const Icon(Icons.refresh_rounded, size: 14, color: ObsidianColors.error),
                  label: Text('Retry', style: ObsidianTypography.labelBadge(color: ObsidianColors.error)),
                  style: TextButton.styleFrom(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                    minimumSize: Size.zero,
                    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  ),
                )
              else
                TextButton.icon(
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(
                        content: Text('Inspecting event details: ${event.message}'),
                        backgroundColor: ObsidianColors.surfaceContainerHigh,
                      ),
                    );
                  },
                  icon: const Icon(Icons.arrow_forward_rounded, size: 14, color: ObsidianColors.primary),
                  label: Text('Inspect', style: ObsidianTypography.labelBadge(color: ObsidianColors.primary)),
                  style: TextButton.styleFrom(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                    minimumSize: Size.zero,
                    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  ),
                ),
            ],
          ),
        ],
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
              IconButton(
                onPressed: null,
                icon: const Icon(Icons.chevron_left_rounded, color: ObsidianColors.outline),
                iconSize: 20,
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('1 / 1', style: ObsidianTypography.labelBadge(color: ObsidianColors.primary)),
              ),
              IconButton(
                onPressed: null,
                icon: const Icon(Icons.chevron_right_rounded, color: ObsidianColors.outline),
                iconSize: 20,
              ),
            ],
          ),
        ],
      ),
    );
  }
}
