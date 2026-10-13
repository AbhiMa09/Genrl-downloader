import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/queue_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';

class QueuePage extends StatefulWidget {
  const QueuePage({super.key});

  @override
  State<QueuePage> createState() => _QueuePageState();
}

class _QueuePageState extends State<QueuePage> {
  String _selectedFilter = 'All';

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    final filteredItems = state.queue.where((item) {
      if (_selectedFilter == 'Downloading') {
        return item.state == QueueState.downloading;
      }
      if (_selectedFilter == 'Completed') {
        return item.state == QueueState.completed ||
            item.state == QueueState.finalizing;
      }
      if (_selectedFilter == 'Stalled / Issues') {
        return item.state == QueueState.stalled ||
            item.state == QueueState.blocked;
      }
      return true;
    }).toList();

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Filter Tabs and Global Control Actions
          Row(
            children: [
              _FilterTab(
                label: 'All (${state.queue.length})',
                isSelected: _selectedFilter == 'All',
                onTap: () => setState(() => _selectedFilter = 'All'),
              ),
              const SizedBox(width: 8),
              _FilterTab(
                label: 'Downloading (${state.activeQueueCount})',
                isSelected: _selectedFilter == 'Downloading',
                onTap: () => setState(() => _selectedFilter = 'Downloading'),
              ),
              const SizedBox(width: 8),
              _FilterTab(
                label: 'Completed',
                isSelected: _selectedFilter == 'Completed',
                onTap: () => setState(() => _selectedFilter = 'Completed'),
              ),
              const SizedBox(width: 8),
              _FilterTab(
                label: 'Stalled / Issues',
                isSelected: _selectedFilter == 'Stalled / Issues',
                onTap: () => setState(() => _selectedFilter = 'Stalled / Issues'),
              ),
              const Spacer(),

              // Global Pause/Resume
              OutlinedButton.icon(
                onPressed: () => state.toggleAutomation(),
                icon: Icon(
                  state.status.automationOn
                      ? Icons.pause_rounded
                      : Icons.play_arrow_rounded,
                  size: 16,
                  color: ObsidianColors.primary,
                ),
                label: Text(
                  state.status.automationOn ? 'Pause Queue' : 'Resume Queue',
                  style: ObsidianTypography.bodySm(color: ObsidianColors.primary),
                ),
                style: OutlinedButton.styleFrom(
                  side: const BorderSide(color: ObsidianColors.border),
                  padding:
                      const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 20),

          // Transfers List
          if (filteredItems.isEmpty)
            Container(
              padding: const EdgeInsets.all(48),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLow,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: ObsidianColors.border),
              ),
              child: Center(
                child: Text(
                  'No torrent transfers in this view category.',
                  style: ObsidianTypography.bodyMd(
                      color: ObsidianColors.outline),
                ),
              ),
            )
          else
            ...filteredItems.map(
              (item) => _DetailedQueueCard(
                item: item,
                onDelete: () => state.removeQueueItem(item.id),
              ),
            ),
        ],
      ),
    );
  }
}

class _FilterTab extends StatelessWidget {
  final String label;
  final bool isSelected;
  final VoidCallback onTap;

  const _FilterTab({
    required this.label,
    required this.isSelected,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: BoxDecoration(
          color: isSelected
              ? ObsidianColors.surfaceContainerHigh
              : ObsidianColors.surfaceContainerLowest,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isSelected
                ? ObsidianColors.primary.withOpacity(0.3)
                : ObsidianColors.border,
          ),
        ),
        child: Text(
          label,
          style: isSelected
              ? ObsidianTypography.bodySm(color: ObsidianColors.primary)
                  .copyWith(fontWeight: FontWeight.w600)
              : ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
        ),
      ),
    );
  }
}

class _DetailedQueueCard extends StatelessWidget {
  final QueueItem item;
  final VoidCallback onDelete;

  const _DetailedQueueCard({
    required this.item,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border, width: 1),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2.5),
                decoration: BoxDecoration(
                  color: ObsidianColors.primary.withOpacity(0.15),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  item.episodeLabel,
                  style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.primary),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  item.showTitle,
                  style: ObsidianTypography.headlineMd().copyWith(fontSize: 16),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              Text(
                '${(item.progress * 100).toStringAsFixed(1)}%',
                style: ObsidianTypography.monoMetric(
                  color: item.progress >= 1.0
                      ? ObsidianColors.success
                      : ObsidianColors.primary,
                ),
              ),
              const SizedBox(width: 12),
              IconButton(
                onPressed: onDelete,
                icon: const Icon(Icons.delete_outline_rounded,
                    size: 18, color: ObsidianColors.outline),
                tooltip: 'Cancel and delete transfer',
                padding: EdgeInsets.zero,
                constraints: const BoxConstraints(),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            item.releaseName,
            style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          const SizedBox(height: 12),

          // Telemetry row
          Row(
            children: [
              _TelemetryBadge(
                icon: Icons.speed_rounded,
                label: item.formattedSpeed,
                color: ObsidianColors.primary,
              ),
              const SizedBox(width: 16),
              _TelemetryBadge(
                icon: Icons.timer_outlined,
                label: 'ETA: ${item.formattedEta}',
                color: ObsidianColors.onSurfaceVariant,
              ),
              const SizedBox(width: 16),
              _TelemetryBadge(
                icon: Icons.folder_open_outlined,
                label: item.formattedSize,
                color: ObsidianColors.onSurfaceVariant,
              ),
              const SizedBox(width: 16),
              _TelemetryBadge(
                icon: Icons.people_outline_rounded,
                label: '${item.seeders} seeds / ${item.leechers} leech',
                color: ObsidianColors.tertiary,
              ),
            ],
          ),
          const SizedBox(height: 10),

          // Linear progress bar
          ClipRRect(
            borderRadius: BorderRadius.circular(4),
            child: LinearProgressIndicator(
              value: item.progress,
              backgroundColor: ObsidianColors.surfaceContainerHighest,
              valueColor: AlwaysStoppedAnimation<Color>(
                item.progress >= 1.0
                    ? ObsidianColors.success
                    : ObsidianColors.primaryContainer,
              ),
              minHeight: 6,
            ),
          ),
        ],
      ),
    );
  }
}

class _TelemetryBadge extends StatelessWidget {
  final IconData icon;
  final String label;
  final Color color;

  const _TelemetryBadge({
    required this.icon,
    required this.label,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 14, color: color),
        const SizedBox(width: 5),
        Text(
          label,
          style: ObsidianTypography.monoCode(color: color),
        ),
      ],
    );
  }
}
