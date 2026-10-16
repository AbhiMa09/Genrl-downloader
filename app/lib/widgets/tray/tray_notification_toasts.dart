import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class TrayNotificationToasts extends StatelessWidget {
  final StateProvider state;
  final VoidCallback onClose;

  const TrayNotificationToasts({
    super.key,
    required this.state,
    required this.onClose,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        // TOAST 1: Download / Activity Toast
        Builder(
          builder: (context) {
            final latestDone = state.events.isNotEmpty
                ? state.events.first
                : null;
            final toastTitle = latestDone != null
                ? latestDone.message
                : (state.shows.isNotEmpty
                      ? '${state.shows.first.title} up to date'
                      : 'All monitored shows up to date');
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
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(
                        content: Text('Playing episode in external player'),
                      ),
                    );
                  },
                  icon: const Icon(Icons.play_arrow_rounded, size: 14),
                  label: const Text('Play file'),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: ObsidianColors.surfaceContainerHigh,
                    foregroundColor: ObsidianColors.onSurface,
                    elevation: 0,
                    padding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 4,
                    ),
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
                    padding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 4,
                    ),
                  ),
                ),
              ],
            );
          },
        ),
        const SizedBox(height: 10),

        // TOAST 2: Inbox Triage Questions
        if (state.inboxCount > 0) ...[
          _buildToast(
            accentColor: ObsidianColors.primary,
            icon: Icons.help_center_rounded,
            iconColor: ObsidianColors.primary,
            category: 'Inbox Triage • Action Required',
            title:
                '${state.inboxCount} question${state.inboxCount > 1 ? 's' : ''} need your input',
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
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 4,
                  ),
                ),
              ),
              const SizedBox(width: 6),
              TextButton(
                onPressed: () {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(
                      content: Text(
                        'Auto-resolving inbox items based on rules',
                      ),
                    ),
                  );
                },
                child: const Text('Auto-resolve with rules'),
              ),
            ],
          ),
          const SizedBox(height: 10),
        ],

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
                padding: const EdgeInsets.symmetric(
                  horizontal: 10,
                  vertical: 4,
                ),
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
                padding: const EdgeInsets.symmetric(
                  horizontal: 10,
                  vertical: 4,
                ),
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
                                Text(
                                  category,
                                  style: ObsidianTypography.labelBadge(
                                    color: ObsidianColors.outline,
                                  ),
                                ),
                                Text(
                                  title,
                                  style: ObsidianTypography.bodySm().copyWith(
                                    fontWeight: FontWeight.bold,
                                  ),
                                ),
                              ],
                            ),
                          ),
                          IconButton(
                            icon: const Icon(
                              Icons.close_rounded,
                              size: 16,
                              color: ObsidianColors.outline,
                            ),
                            onPressed: () {},
                          ),
                        ],
                      ),
                      const SizedBox(height: 6),
                      Text(
                        body,
                        style: ObsidianTypography.labelBadge(
                          color: ObsidianColors.onSurfaceVariant,
                        ),
                      ),
                      const SizedBox(height: 8),
                      Wrap(spacing: 6, runSpacing: 4, children: actions),
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
