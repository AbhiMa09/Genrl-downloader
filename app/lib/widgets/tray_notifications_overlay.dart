import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';

class SystemTrayNotificationOverlay extends StatelessWidget {
  final VoidCallback onClose;

  const SystemTrayNotificationOverlay({super.key, required this.onClose});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    return Material(
      color: Colors.transparent,
      child: Stack(
        children: [
          // Dismissible background barrier
          Positioned.fill(
            child: GestureDetector(
              onTap: onClose,
              child: Container(color: Colors.black45),
            ),
          ),

          // Top right floating container matching prototype
          Positioned(
            top: 60,
            right: 24,
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 820),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // LEFT: System Tray Popup Menu
                  SizedBox(
                    width: 330,
                    child: _buildTrayMenu(context, state),
                  ),
                  const SizedBox(width: 16),

                  // RIGHT: Desktop Notification Toasts (Stack of 3)
                  SizedBox(
                    width: 420,
                    child: _buildNotificationToasts(context, state),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildTrayMenu(BuildContext context, StateProvider state) {
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
                Row(
                  children: [
                    const Icon(Icons.downloading_rounded, size: 18, color: ObsidianColors.primary),
                    const SizedBox(width: 8),
                    Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text('Genrl Downloader', style: ObsidianTypography.titleSm().copyWith(fontWeight: FontWeight.bold)),
                        Text('Background Engine', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                      ],
                    ),
                  ],
                ),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                    color: ObsidianColors.tertiary.withOpacity(0.18),
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Row(
                    children: [
                      Container(width: 5, height: 5, decoration: const BoxDecoration(shape: BoxShape.circle, color: ObsidianColors.tertiary)),
                      const SizedBox(width: 4),
                      Text('Active', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
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
                    child: const Icon(Icons.sync_rounded, size: 16, color: ObsidianColors.tertiary),
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
                              TextSpan(text: '$downloadingCount downloading', style: const TextStyle(color: ObsidianColors.tertiary, fontWeight: FontWeight.bold)),
                              const TextSpan(text: ', '),
                              TextSpan(text: '$alertCount need input', style: const TextStyle(color: ObsidianColors.secondary, fontWeight: FontWeight.bold)),
                            ],
                          ),
                        ),
                        Text('DL: 42.8 MB/s • UL: 3.1 MB/s', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                      ],
                    ),
                  ),
                  if (alertCount > 0)
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                      decoration: BoxDecoration(
                        color: ObsidianColors.error.withOpacity(0.2),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text('$alertCount', style: ObsidianTypography.labelBadge(color: ObsidianColors.error).copyWith(fontWeight: FontWeight.bold)),
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
                    ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('All transfers paused')));
                    onClose();
                  },
                ),
                _buildMenuItem(
                  icon: Icons.play_circle_outline_rounded,
                  iconColor: ObsidianColors.outline,
                  label: 'Resume',
                  subtext: 'Active (2 running)',
                  onTap: () {
                    ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Transfers resumed')));
                    onClose();
                  },
                ),
                _buildMenuItem(
                  icon: Icons.sync_rounded,
                  iconColor: ObsidianColors.outline,
                  label: 'Sync now',
                  subtext: 'MAL • Tokyo Toshokan',
                  onTap: () {
                    state.syncNow();
                    onClose();
                  },
                ),
                const Divider(color: ObsidianColors.border, height: 12),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceAround,
                  children: [
                    TextButton.icon(
                      onPressed: () {
                        state.selectPage(7); // Settings
                        onClose();
                      },
                      icon: const Icon(Icons.tune_rounded, size: 14),
                      label: const Text('Settings'),
                    ),
                    TextButton.icon(
                      onPressed: () {
                        state.selectPage(1); // Inbox
                        onClose();
                      },
                      icon: const Icon(Icons.inbox_rounded, size: 14),
                      label: Text('Inbox ($alertCount)'),
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
                Text('Daemon: Unix Socket OK', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                Text('RAM: 38.4 MB', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
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
          color: isPrimary ? ObsidianColors.primary.withOpacity(0.12) : Colors.transparent,
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
                  color: isError ? ObsidianColors.error : (isPrimary ? ObsidianColors.primary : ObsidianColors.onSurface),
                ).copyWith(fontWeight: isPrimary ? FontWeight.bold : FontWeight.normal),
              ),
            ),
            if (shortcut != null)
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(shortcut, style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
              ),
            if (subtext != null)
              Text(subtext, style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
          ],
        ),
      ),
    );
  }

  Widget _buildNotificationToasts(BuildContext context, StateProvider state) {
    return Column(
      children: [
        // TOAST 1: Download / Activity Toast
        Builder(builder: (context) {
          final latestDone = state.events.isNotEmpty ? state.events.first : null;
          final toastTitle = latestDone != null
              ? latestDone.message
              : (state.shows.isNotEmpty ? '${state.shows.first.title} up to date' : 'All monitored shows up to date');
          final toastBody = latestDone != null
              ? 'Event logged at ${latestDone.formattedTime}'
              : 'Background scanner idle. No active transfers.';

          return _buildToast(
            accentColor: ObsidianColors.tertiary,
            icon: Icons.download_done_rounded,
            iconColor: ObsidianColors.tertiary,
            category: 'Genrl Downloader • Core',
            title: toastTitle,
            body: toastBody,
            actions: [
              ElevatedButton.icon(
                onPressed: () {
                  ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Playing episode in external player')));
                },
                icon: const Icon(Icons.play_arrow_rounded, size: 14),
                label: const Text('Play file'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainerHigh,
                  foregroundColor: ObsidianColors.onSurface,
                  elevation: 0,
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                ),
              ),
              const SizedBox(width: 6),
              OutlinedButton.icon(
                onPressed: () {},
                icon: const Icon(Icons.folder_open_rounded, size: 14),
                label: const Text('Show in folder'),
                style: OutlinedButton.styleFrom(
                  side: const BorderSide(color: ObsidianColors.border),
                  foregroundColor: ObsidianColors.onSurfaceVariant,
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                ),
              ),
            ],
          );
        }),
        const SizedBox(height: 10),

        // TOAST 2: Inbox Triage Questions
        if (state.inboxCount > 0)
          _buildToast(
            accentColor: ObsidianColors.primary,
            icon: Icons.help_center_rounded,
            iconColor: ObsidianColors.primary,
            category: 'Inbox Triage • Action Required',
            title: '${state.inboxCount} question${state.inboxCount > 1 ? 's' : ''} need your input',
            body: 'Candidate ties or ambiguous match questions are waiting for resolution in your inbox.',
            actions: [
              ElevatedButton.icon(
              onPressed: () {
                state.selectPage(1); // Inbox
                onClose();
              },
              icon: const Icon(Icons.inbox_rounded, size: 14),
              label: const Text('Open Inbox'),
              style: ElevatedButton.styleFrom(
                backgroundColor: ObsidianColors.primary,
                foregroundColor: ObsidianColors.onPrimary,
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
              ),
            ),
            const SizedBox(width: 6),
            TextButton(
              onPressed: () {
                ScaffoldMessenger.of(context).showSnackBar(const SnackBar(content: Text('Auto-resolving inbox items based on rules')));
              },
              child: const Text('Auto-resolve with rules'),
            ),
          ],
        ),
        const SizedBox(height: 10),

        // TOAST 3: RPC Failure Alert
        _buildToast(
          accentColor: ObsidianColors.error,
          icon: Icons.cloud_off_rounded,
          iconColor: ObsidianColors.error,
          category: 'RPC Bridge Failure • Urgent',
          title: 'qBittorrent is not reachable',
          body: 'Connection refused at http://127.0.0.1:8080. Torrent downloads and magnet handoffs are temporarily queued locally.',
          actions: [
            ElevatedButton.icon(
              onPressed: () => state.refreshAll(),
              icon: const Icon(Icons.refresh_rounded, size: 14),
              label: const Text('Retry'),
              style: ElevatedButton.styleFrom(
                backgroundColor: ObsidianColors.error.withOpacity(0.2),
                foregroundColor: ObsidianColors.error,
                elevation: 0,
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
              ),
            ),
            const SizedBox(width: 6),
            OutlinedButton.icon(
              onPressed: () {
                state.selectPage(7); // Settings
                onClose();
              },
              icon: const Icon(Icons.settings_rounded, size: 14),
              label: const Text('Check settings'),
              style: OutlinedButton.styleFrom(
                side: const BorderSide(color: ObsidianColors.border),
                foregroundColor: ObsidianColors.onSurfaceVariant,
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildToast({
    required Color accentColor,
    required IconData icon,
    required Color iconColor,
    required String category,
    required String title,
    required String body,
    required List<Widget> actions,
  }) {
    return Container(
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow.withOpacity(0.96),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: ObsidianColors.border),
        boxShadow: const [
          BoxShadow(
            color: Colors.black45,
            blurRadius: 16,
            offset: Offset(0, 6),
          ),
        ],
      ),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(16),
        child: IntrinsicHeight(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // Left Accent Line
              Container(width: 5, color: accentColor),

              // Content Area
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Container(
                            width: 32,
                            height: 32,
                            decoration: BoxDecoration(
                              color: iconColor.withOpacity(0.18),
                              borderRadius: BorderRadius.circular(8),
                            ),
                            child: Icon(icon, size: 18, color: iconColor),
                          ),
                          const SizedBox(width: 10),
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(category, style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                                Text(title, style: ObsidianTypography.bodySm().copyWith(fontWeight: FontWeight.bold)),
                              ],
                            ),
                          ),
                          IconButton(
                            icon: const Icon(Icons.close_rounded, size: 16, color: ObsidianColors.outline),
                            onPressed: () {},
                          ),
                        ],
                      ),
                      const SizedBox(height: 6),
                      Text(body, style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurfaceVariant)),
                      const SizedBox(height: 8),
                      Row(children: actions),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
