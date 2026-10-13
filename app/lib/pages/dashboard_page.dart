import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/queue_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/metric_card.dart';
import '../widgets/anime_poster.dart';
import '../widgets/state_views.dart';

class DashboardPage extends StatelessWidget {
  const DashboardPage({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();
    final status = state.status;

    if (state.isLoadingStatus && state.queue.isEmpty && state.shows.isEmpty && state.coreError == null) {
      return const Center(
        child: LoadingStateView(message: 'Querying system telemetry from genrl core...'),
      );
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (state.coreError != null)
            ErrorStateView(
              title: 'Core Engine Unreachable',
              message: state.coreError!,
              onRetry: () => state.refreshAll(),
              retryLabel: 'Retry Connection',
            ),
          // 1. Action & Command Bar
          Container(
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLow,
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: ObsidianColors.border, width: 1),
            ),
            child: Wrap(
              alignment: WrapAlignment.spaceBetween,
              crossAxisAlignment: WrapCrossAlignment.center,
              spacing: 12,
              runSpacing: 10,
              children: [
                // Quick info indicator
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: 8,
                      height: 8,
                      decoration: const BoxDecoration(
                        color: ObsidianColors.tertiary,
                        shape: BoxShape.circle,
                      ),
                    ),
                    const SizedBox(width: 8),
                    Text(
                      'qBittorrent ${status.qbitVersion} :: IPC Linked',
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),

                // Action buttons
                Wrap(
                  spacing: 10,
                  runSpacing: 10,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    _ActionButton(
                      icon: Icons.sync_rounded,
                      label: 'Force Sync MAL',
                      iconColor: ObsidianColors.primary,
                      isLoading: state.isSyncing,
                      onTap: () => state.forceSyncMal(),
                    ),
                    _ActionButton(
                      icon: Icons.folder_shared_outlined,
                      label: 'Manual Disk Scan',
                      iconColor: ObsidianColors.tertiary,
                      isLoading: state.isScanning,
                      onTap: () => state.manualDiskScan(),
                    ),
                    ElevatedButton.icon(
                      onPressed: () => state.selectPage(4), // Go to search
                      icon: const Icon(Icons.bolt_rounded, size: 16),
                      label: const Text('Grab Anime'),
                      style: ElevatedButton.styleFrom(
                        backgroundColor: ObsidianColors.primaryContainer,
                        foregroundColor: ObsidianColors.onPrimaryContainer,
                        elevation: 0,
                        padding: const EdgeInsets.symmetric(
                            horizontal: 14, vertical: 12),
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(8),
                        ),
                        textStyle: ObsidianTypography.titleSm(),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),

          // 2. Health & State Pulse Bar
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: ObsidianColors.border, width: 0.5),
            ),
            child: Wrap(
              alignment: WrapAlignment.spaceBetween,
              crossAxisAlignment: WrapCrossAlignment.center,
              spacing: 12,
              runSpacing: 8,
              children: [
                Wrap(
                  spacing: 12,
                  runSpacing: 6,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        const Icon(Icons.download_rounded,
                            size: 15, color: ObsidianColors.tertiary),
                        const SizedBox(width: 6),
                        Text(
                          '${status.activeTorrents} Active Torrents',
                          style: ObsidianTypography.monoMetric(
                            color: ObsidianColors.onSurface,
                          ),
                        ),
                      ],
                    ),
                    const Text('/', style: TextStyle(color: ObsidianColors.outline)),
                    Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        const Icon(Icons.bookmark_added_outlined,
                            size: 15, color: ObsidianColors.primary),
                        const SizedBox(width: 6),
                        Text(
                          '${status.totalShows} Monitored Shows',
                          style: ObsidianTypography.monoCode(
                            color: ObsidianColors.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                    const Text('/', style: TextStyle(color: ObsidianColors.outline)),
                    Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        const Icon(Icons.update_rounded,
                            size: 15, color: ObsidianColors.secondary),
                        const SizedBox(width: 6),
                        Text(
                          state.shows.isNotEmpty
                              ? 'Latest: ${state.shows.first.title}'
                              : 'No shows indexed yet',
                          style: ObsidianTypography.monoCode(
                            color: ObsidianColors.primaryFixed,
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    'CRON: ACTIVE',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 20),

          // 3. Four KPI Metric Cards
          Row(
            children: [
              Expanded(
                child: MetricCard(
                  title: 'Pipeline Flow',
                  icon: Icons.cloud_download_outlined,
                  primaryValue: '${status.activeTorrents}',
                  subtitle: 'Active',
                  detailLine:
                      '${status.formattedDownloadSpeed} • ETA ~14m',
                  progress: 0.68,
                  accentColor: ObsidianColors.primary,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: MetricCard(
                  title: 'Library Index',
                  icon: Icons.video_library_outlined,
                  primaryValue: '${status.totalShows}',
                  subtitle: 'Series',
                  detailLine:
                      '${status.totalEpisodes} Ep • ${status.totalDiskSpaceGb ~/ 1000} TB library',
                  progress: 0.85,
                  accentColor: ObsidianColors.tertiary,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: MetricCard(
                  title: 'Inbox Attention',
                  icon: Icons.inbox_outlined,
                  primaryValue: '${state.inboxCount}',
                  subtitle: 'Questions',
                  detailLine: 'Collisions & tiebreaks pending',
                  progress: state.inboxCount > 0 ? 1.0 : 0.0,
                  accentColor: state.inboxCount > 0
                      ? ObsidianColors.error
                      : ObsidianColors.success,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: MetricCard(
                  title: 'Storage & Health',
                  icon: Icons.storage_rounded,
                  primaryValue: '${status.freeDiskSpaceGb} GB',
                  subtitle: 'Free',
                  detailLine: 'Circuit breakers: all closed',
                  progress: 0.42,
                  accentColor: ObsidianColors.success,
                ),
              ),
            ],
          ),
          const SizedBox(height: 24),

          // 4. Active Pipeline Table
          Container(
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
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(
                      'ACTIVE DOWNLOAD TRANSFERS',
                      style: ObsidianTypography.titleSm(),
                    ),
                    InkWell(
                      onTap: () => state.selectPage(2), // View all queue
                      child: Text(
                        'View Full Queue →',
                        style: ObsidianTypography.bodySm(
                            color: ObsidianColors.primary),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 14),

                if (state.queue.isEmpty)
                  Padding(
                    padding: const EdgeInsets.all(24),
                    child: Center(
                      child: Text(
                        'Queue is empty. Automation is watching for newly aired releases.',
                        style: ObsidianTypography.bodyMd(
                            color: ObsidianColors.outline),
                      ),
                    ),
                  )
                else
                  ...state.queue.map((item) => _QueueRow(item: item)),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // 5. Recently Filed in Library
          Container(
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
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(
                      'RECENTLY ORGANIZED ANIME',
                      style: ObsidianTypography.titleSm(),
                    ),
                    InkWell(
                      onTap: () => state.selectPage(3), // View library
                      child: Text(
                        'Browse Library →',
                        style: ObsidianTypography.bodySm(
                            color: ObsidianColors.primary),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 14),

                if (state.shows.isEmpty)
                  Padding(
                    padding: const EdgeInsets.all(24),
                    child: Center(
                      child: Text(
                        'No organized shows discovered yet. Run a disk scan or connect MAL to populate.',
                        style: ObsidianTypography.bodyMd(
                            color: ObsidianColors.outline),
                      ),
                    ),
                  )
                else
                  Row(
                    children: state.shows.take(5).map((show) {
                      return Expanded(
                        child: Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 6),
                          child: InkWell(
                            onTap: () => state.openShowDetail(show),
                            borderRadius: BorderRadius.circular(8),
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                // Anime Poster Thumbnail (2:3 aspect ratio)
                                AnimePoster(
                                  urlOrPath: show.posterUrl,
                                  aspectRatio: 2 / 3,
                                  borderRadius: BorderRadius.circular(8),
                                ),
                                const SizedBox(height: 8),
                                Text(
                                  show.title,
                                  style: ObsidianTypography.bodySm(
                                      color: ObsidianColors.onSurface),
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                ),
                                Text(
                                  '${show.downloadedEpisodes}/${show.totalEpisodes} Ep • ${show.startYear}',
                                  style: ObsidianTypography.monoCode(
                                      color: ObsidianColors.outline),
                                ),
                              ],
                            ),
                          ),
                        ),
                      );
                    }).toList(),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _ActionButton extends StatelessWidget {
  final IconData icon;
  final String label;
  final Color iconColor;
  final bool isLoading;
  final VoidCallback onTap;

  const _ActionButton({
    required this.icon,
    required this.label,
    required this.iconColor,
    this.isLoading = false,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: isLoading ? null : onTap,
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainer,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: ObsidianColors.border, width: 0.5),
        ),
        child: Row(
          children: [
            if (isLoading)
              const SizedBox(
                width: 14,
                height: 14,
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  valueColor: AlwaysStoppedAnimation<Color>(ObsidianColors.primary),
                ),
              )
            else
              Icon(icon, size: 16, color: iconColor),
            const SizedBox(width: 8),
            Text(label, style: ObsidianTypography.bodySm()),
          ],
        ),
      ),
    );
  }
}

class _QueueRow extends StatelessWidget {
  final QueueItem item;

  const _QueueRow({required this.item});

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.only(bottom: 10),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: ObsidianColors.border, width: 0.5),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
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
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  item.showTitle,
                  style: ObsidianTypography.titleSm(),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              Text(
                item.formattedSpeed,
                style: ObsidianTypography.monoMetric(
                    color: ObsidianColors.primary),
              ),
              const SizedBox(width: 12),
              Text(
                'ETA: ${item.formattedEta}',
                style: ObsidianTypography.monoCode(
                    color: ObsidianColors.onSurfaceVariant),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            item.releaseName,
            style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          const SizedBox(height: 8),
          ClipRRect(
            borderRadius: BorderRadius.circular(3),
            child: LinearProgressIndicator(
              value: item.progress,
              backgroundColor: ObsidianColors.surfaceContainerHighest,
              valueColor: const AlwaysStoppedAnimation<Color>(
                  ObsidianColors.primaryContainer),
              minHeight: 5,
            ),
          ),
        ],
      ),
    );
  }
}
