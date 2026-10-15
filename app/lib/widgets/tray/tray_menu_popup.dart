import 'package:flutter/material.dart';
import '../../models/queue_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class TrayMenuPopup extends StatelessWidget {
  final StateProvider state;
  final VoidCallback onClose;

  const TrayMenuPopup({
    super.key,
    required this.state,
    required this.onClose,
  });

  @override
  Widget build(BuildContext context) {
    final downloadingCount = state.activeDownloads.length;
    final alertCount = state.inboxCount;

    return Container(
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow.withOpacity(0.96),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: ObsidianColors.border),
        boxShadow: const [
          BoxShadow(
            color: Colors.black54,
            blurRadius: 24,
            offset: Offset(0, 10),
          ),
        ],
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Header
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
            decoration: const BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Expanded(
                  child: Row(
                    children: [
                      const Icon(Icons.downloading_rounded,
                          size: 18, color: ObsidianColors.primary),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text('Genrl Downloader',
                                style: ObsidianTypography.titleSm()
                                    .copyWith(fontWeight: FontWeight.bold),
                                overflow: TextOverflow.ellipsis),
                            Text('Background Engine',
                                style: ObsidianTypography.labelBadge(
                                    color: ObsidianColors.outline),
                                overflow: TextOverflow.ellipsis),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                    color: ObsidianColors.tertiary.withOpacity(0.18),
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Row(
                    children: [
                      Container(
                          width: 5,
                          height: 5,
                          decoration: const BoxDecoration(
                              shape: BoxShape.circle,
                              color: ObsidianColors.tertiary)),
                      const SizedBox(width: 4),
                      Text('Active',
                          style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.tertiary)),
                    ],
                  ),
                ),
              ],
            ),
          ),

          // Highlighted Status Metric Pill
          Padding(
            padding: const EdgeInsets.all(12),
            child: Container(
              padding: const EdgeInsets.all(10),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerHigh,
                borderRadius: BorderRadius.circular(10),
                border: Border.all(color: ObsidianColors.border),
              ),
              child: Row(
                children: [
                  Container(
                    width: 28,
                    height: 28,
                    decoration: BoxDecoration(
                      color: ObsidianColors.tertiary.withOpacity(0.2),
                      borderRadius: BorderRadius.circular(6),
                    ),
                    child: const Icon(Icons.sync_rounded,
                        size: 16, color: ObsidianColors.tertiary),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        RichText(
                          text: TextSpan(
                            style: ObsidianTypography.bodySm(),
                            children: [
                              TextSpan(
                                  text: '$downloadingCount downloading',
                                  style: const TextStyle(
                                      color: ObsidianColors.tertiary,
                                      fontWeight: FontWeight.bold)),
                              const TextSpan(text: ', '),
                              TextSpan(
                                  text: '$alertCount need input',
                                  style: const TextStyle(
                                      color: ObsidianColors.secondary,
                                      fontWeight: FontWeight.bold)),
                            ],
                          ),
                        ),
                        Builder(
                          builder: (context) {
                            double totalDl = 0;
                            double totalUl = 0;
                            for (final q in state.queue) {
                              if (q.state == QueueState.downloading) {
                                totalDl += q.speedBytes;
                                totalUl += q.uploadSpeedBytes;
                              }
                            }
                            final dlText = totalDl >= 1024 * 1024
                                ? '${(totalDl / (1024 * 1024)).toStringAsFixed(1)} MB/s'
                                : (totalDl > 0
                                    ? '${(totalDl / 1024).toStringAsFixed(1)} KB/s'
                                    : '0 B/s');
                            final ulText = totalUl >= 1024 * 1024
                                ? '${(totalUl / (1024 * 1024)).toStringAsFixed(1)} MB/s'
                                : (totalUl > 0
                                    ? '${(totalUl / 1024).toStringAsFixed(1)} KB/s'
                                    : '0 B/s');
                            return Text('DL: $dlText • UL: $ulText',
                                style: ObsidianTypography.labelBadge(
                                    color: ObsidianColors.outline));
                          },
                        ),
                      ],
                    ),
                  ),
                  if (alertCount > 0)
                    Container(
                      padding: const EdgeInsets.symmetric(
                          horizontal: 6, vertical: 2),
                      decoration: BoxDecoration(
                        color: ObsidianColors.error.withOpacity(0.2),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text('$alertCount',
                          style: ObsidianTypography.labelBadge(
                                  color: ObsidianColors.error)
                              .copyWith(fontWeight: FontWeight.bold)),
                    ),
                ],
              ),
            ),
          ),

          // Menu Items List
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: Column(
              children: [
                _buildMenuItem(
                  icon: Icons.open_in_new_rounded,
                  iconColor: ObsidianColors.primary,
                  label: 'Open Genrl Downloader',
                  shortcut: 'Super+G',
                  isPrimary: true,
                  onTap: onClose,
                ),
                _buildMenuItem(
                  icon: Icons.pause_circle_outline_rounded,
                  iconColor: ObsidianColors.outline,
                  label: 'Pause all',
                  subtext: 'Hold transfers',
                  onTap: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                        const SnackBar(content: Text('All transfers paused')));
                    onClose();
                  },
                ),
                _buildMenuItem(
                  icon: Icons.play_circle_outline_rounded,
                  iconColor: ObsidianColors.outline,
                  label: 'Resume',
                  subtext: 'Active (2 running)',
                  onTap: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                        const SnackBar(content: Text('Transfers resumed')));
                    onClose();
                  },
                ),
                _buildMenuItem(
                  icon: Icons.sync_rounded,
                  iconColor: ObsidianColors.outline,
                  label: 'Sync now',
                  subtext: 'MAL • Nyaa.si',
                  onTap: () {
                    state.syncNow();
                    onClose();
                  },
                ),
                const Divider(color: ObsidianColors.border, height: 12),
                Row(
                  children: [
                    Expanded(
                      child: TextButton.icon(
                        onPressed: () {
                          state.selectPage(7); // Settings
                          onClose();
                        },
                        icon: const Icon(Icons.tune_rounded, size: 14),
                        label: const Text('Settings',
                            overflow: TextOverflow.ellipsis),
                      ),
                    ),
                    Expanded(
                      child: TextButton.icon(
                        onPressed: () {
                          state.selectPage(1); // Inbox
                          onClose();
                        },
                        icon: const Icon(Icons.inbox_rounded, size: 14),
                        label: Text('Inbox ($alertCount)',
                            overflow: TextOverflow.ellipsis),
                      ),
                    ),
                  ],
                ),
                const Divider(color: ObsidianColors.border, height: 12),
                _buildMenuItem(
                  icon: Icons.power_settings_new_rounded,
                  iconColor: ObsidianColors.error,
                  label: 'Quit',
                  shortcut: 'Ctrl+Q',
                  isError: true,
                  onTap: onClose,
                ),
              ],
            ),
          ),
          const SizedBox(height: 4),

          // Footer Monospace Subtext Note
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
            decoration: const BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.vertical(bottom: Radius.circular(16)),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Flexible(
                    child: Text('Daemon: Unix Socket OK',
                        style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline),
                        overflow: TextOverflow.ellipsis)),
                const SizedBox(width: 4),
                Flexible(
                    child: Text('RAM: 38.4 MB',
                        style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline),
                        overflow: TextOverflow.ellipsis)),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildMenuItem({
    required IconData icon,
    required Color iconColor,
    required String label,
    String? shortcut,
    String? subtext,
    bool isPrimary = false,
    bool isError = false,
    required VoidCallback onTap,
  }) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
        decoration: BoxDecoration(
          color: isPrimary
              ? ObsidianColors.primary.withOpacity(0.12)
              : Colors.transparent,
          borderRadius: BorderRadius.circular(8),
        ),
        child: Row(
          children: [
            Icon(icon, size: 18, color: iconColor),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                label,
                style: ObsidianTypography.bodySm(
                  color: isError
                      ? ObsidianColors.error
                      : (isPrimary
                          ? ObsidianColors.primary
                          : ObsidianColors.onSurface),
                ).copyWith(
                    fontWeight: isPrimary ? FontWeight.bold : FontWeight.normal),
              ),
            ),
            if (shortcut != null)
              Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(shortcut,
                    style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.outline)),
              ),
            if (subtext != null)
              Text(subtext,
                  style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline)),
          ],
        ),
      ),
    );
  }
}
