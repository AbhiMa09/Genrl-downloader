import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../models/queue_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'queue_phase_pill.dart';

class QueueTableRow extends StatefulWidget {
  final QueueItem item;
  final bool isSelected;
  final VoidCallback onTap;

  const QueueTableRow({
    super.key,
    required this.item,
    required this.isSelected,
    required this.onTap,
  });

  @override
  State<QueueTableRow> createState() => _QueueTableRowState();
}

class _QueueTableRowState extends State<QueueTableRow> {
  bool _isHovered = false;

  Widget _actionBtn({
    required String tooltip,
    required IconData icon,
    required Color color,
    required VoidCallback onTap,
  }) {
    return Tooltip(
      message: tooltip,
      waitDuration: const Duration(milliseconds: 400),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(4),
        child: Padding(
          padding: const EdgeInsets.all(4.0),
          child: Icon(icon, size: 16, color: color),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final item = widget.item;
    final isSelected = widget.isSelected;

    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: InkWell(
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 120),
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          decoration: BoxDecoration(
            color: isSelected
                ? ObsidianColors.surfaceContainer.withOpacity(0.9)
                : (_isHovered
                      ? ObsidianColors.surfaceContainerLow.withOpacity(0.6)
                      : Colors.transparent),
            border: isSelected
                ? const Border(
                    left: BorderSide(color: ObsidianColors.primary, width: 3),
                  )
                : null,
          ),
          child: Row(
            children: [
              // 1. Title & Release (with mini poster thumbnail)
              Expanded(
                flex: 5,
                child: Row(
                  children: [
                    Container(
                      width: 32,
                      height: 48,
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHighest,
                        borderRadius: BorderRadius.circular(4),
                      ),
                      clipBehavior: Clip.antiAlias,
                      child: const Icon(
                        Icons.movie_filter_rounded,
                        size: 16,
                        color: ObsidianColors.outline,
                      ),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Row(
                            children: [
                              Expanded(
                                child: Text(
                                  item.title,
                                  style: ObsidianTypography.titleSm(
                                    color: isSelected
                                        ? ObsidianColors.primary
                                        : ObsidianColors.onSurface,
                                    fontSize: 12,
                                  ),
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 2),
                          Text(
                            '${item.episodeLabel} · ${item.releaseName.isNotEmpty ? item.releaseName : item.group}',
                            style: ObsidianTypography.monoCode(
                              color: ObsidianColors.outline,
                              fontSize: 10,
                            ),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          if (item.error != null && item.error!.isNotEmpty) ...[
                            const SizedBox(height: 2),
                            Text(
                              item.error!,
                              style: ObsidianTypography.monoCode(
                                color: ObsidianColors.error,
                                fontSize: 10,
                              ),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ],
                        ],
                      ),
                    ),
                  ],
                ),
              ),

              // 2. State / Phase
              Expanded(
                flex: 3,
                child: Align(
                  alignment: Alignment.centerLeft,
                  child: QueuePhasePill(item: item),
                ),
              ),

              // 3. Seeders
              Expanded(
                flex: 2,
                child: RichText(
                  text: TextSpan(
                    style: ObsidianTypography.monoCode(fontSize: 11),
                    children: [
                      TextSpan(
                        text: '${item.seeders} seeds',
                        style: TextStyle(
                          color: item.seeders > 10
                              ? ObsidianColors.tertiary
                              : ObsidianColors.error,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      TextSpan(
                        text: ' (${item.leechers} leech)',
                        style: const TextStyle(color: ObsidianColors.outline),
                      ),
                    ],
                  ),
                ),
              ),

              // 4. Progress & Speed
              Expanded(
                flex: 3,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    ClipRRect(
                      borderRadius: BorderRadius.circular(2),
                      child: LinearProgressIndicator(
                        value: item.progress,
                        minHeight: 4,
                        backgroundColor: ObsidianColors.surfaceContainerHighest,
                        valueColor: AlwaysStoppedAnimation(
                          item.state == QueueState.stalled
                              ? ObsidianColors.error
                              : (item.state == QueueState.finalizing
                                    ? ObsidianColors.tertiary
                                    : ObsidianColors.primary),
                        ),
                      ),
                    ),
                    const SizedBox(height: 4),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Flexible(
                          child: Text(
                            item.downloadSpeed,
                            style: ObsidianTypography.monoCode(
                              color: ObsidianColors.primary,
                              fontSize: 10,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        const SizedBox(width: 4),
                        Flexible(
                          child: Text(
                            item.formattedSize,
                            style: ObsidianTypography.monoCode(
                              color: ObsidianColors.outline,
                              fontSize: 9,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),

              // 5. ETA
              Expanded(
                flex: 2,
                child: Text(
                  item.state == QueueState.failed ? 'Failed' : item.eta,
                  textAlign: TextAlign.right,
                  style: ObsidianTypography.monoMetric(
                    color:
                        (item.state == QueueState.stalled ||
                            item.state == QueueState.failed)
                        ? ObsidianColors.error
                        : ObsidianColors.primary,
                    fontSize: 11,
                  ),
                ),
              ),

              // 6. Actions
              SizedBox(
                width: 60,
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    _actionBtn(
                      tooltip: item.state == QueueState.failed
                          ? 'Retry Job'
                          : 'Step Job',
                      icon: item.state == QueueState.failed
                          ? Icons.refresh_rounded
                          : Icons.restart_alt_rounded,
                      color: item.state == QueueState.failed
                          ? ObsidianColors.primary
                          : ObsidianColors.outline,
                      onTap: () async {
                        final p = context.read<StateProvider>();
                        final isFailed = item.state == QueueState.failed;
                        final ok = isFailed
                            ? await p.retryJob(item.id)
                            : await p.recheckJob(item.id);
                        if (context.mounted) {
                          final verb = isFailed ? 'Retried' : 'Stepped';
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(
                              content: Text(
                                ok
                                    ? '$verb job #${item.id}'
                                    : 'Failed to $verb job #${item.id}',
                              ),
                            ),
                          );
                        }
                      },
                    ),
                    _actionBtn(
                      tooltip: 'Cancel Job',
                      icon: Icons.delete_outline_rounded,
                      color: ObsidianColors.outline,
                      onTap: () async {
                        final ok = await context
                            .read<StateProvider>()
                            .cancelJob(item.id);
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(
                              content: Text(
                                ok
                                    ? 'Cancelled job #${item.id}'
                                    : 'Failed to cancel job #${item.id}',
                              ),
                            ),
                          );
                        }
                      },
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
