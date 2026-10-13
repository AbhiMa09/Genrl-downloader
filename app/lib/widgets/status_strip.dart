import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';

class StatusStrip extends StatelessWidget {
  const StatusStrip({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();
    final status = state.status;

    return LayoutBuilder(
      builder: (context, constraints) {
        return Container(
          height: 32,
          padding: const EdgeInsets.symmetric(horizontal: 16),
          decoration: const BoxDecoration(
            color: ObsidianColors.surfaceContainerLowest,
            border: Border(
              top: BorderSide(color: ObsidianColors.border, width: 1),
            ),
          ),
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: ConstrainedBox(
              constraints: BoxConstraints(minWidth: constraints.maxWidth > 32 ? constraints.maxWidth - 32 : 0),
              child: Row(
                children: [
                  // Automation on/off toggle button
                  InkWell(
                    onTap: () => state.toggleAutomation(),
                    borderRadius: BorderRadius.circular(4),
                    child: Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                      child: Row(
                        children: [
                          Container(
                            width: 7,
                            height: 7,
                            decoration: BoxDecoration(
                              color: status.automationOn
                                  ? ObsidianColors.success
                                  : ObsidianColors.warning,
                              shape: BoxShape.circle,
                            ),
                          ),
                          const SizedBox(width: 6),
                          Text(
                            status.automationOn ? 'Automation: Active' : 'Automation: Paused',
                            style: ObsidianTypography.monoCode(
                              color: status.automationOn
                                  ? ObsidianColors.onSurface
                                  : ObsidianColors.warning,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                  const SizedBox(width: 12),
                  const _Separator(),
                  const SizedBox(width: 12),

                  // qBittorrent connection
                  Row(
                    children: [
                      Container(
                        width: 7,
                        height: 7,
                        decoration: BoxDecoration(
                          color: status.qbitConnected
                              ? ObsidianColors.tertiary
                              : ObsidianColors.error,
                          shape: BoxShape.circle,
                        ),
                      ),
                      const SizedBox(width: 6),
                      Text(
                        'qBittorrent ${status.qbitVersion}',
                        style: ObsidianTypography.monoCode(
                          color: ObsidianColors.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(width: 12),
                  const _Separator(),
                  const SizedBox(width: 12),

                  // Total download speed
                  Row(
                    children: [
                      const Icon(
                        Icons.arrow_downward_rounded,
                        size: 13,
                        color: ObsidianColors.primary,
                      ),
                      const SizedBox(width: 4),
                      Text(
                        status.formattedDownloadSpeed,
                        style: ObsidianTypography.monoMetric(
                          color: ObsidianColors.primary,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(width: 24),

                  // Disk Space
                  Row(
                    children: [
                      const Icon(
                        Icons.storage_rounded,
                        size: 13,
                        color: ObsidianColors.outline,
                      ),
                      const SizedBox(width: 4),
                      Text(
                        status.formattedDiskSpace,
                        style: ObsidianTypography.monoCode(
                          color: ObsidianColors.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(width: 12),
                  const _Separator(),
                  const SizedBox(width: 12),

                  // Last MAL sync
                  InkWell(
                    onTap: state.isSyncing ? null : () => state.forceSyncMal(),
                    borderRadius: BorderRadius.circular(4),
                    child: Row(
                      children: [
                        Icon(
                          Icons.sync_rounded,
                          size: 13,
                          color: state.isSyncing
                              ? ObsidianColors.primary
                              : ObsidianColors.outline,
                        ),
                        const SizedBox(width: 4),
                        Text(
                          state.isSyncing ? 'Syncing...' : status.formattedLastSync,
                          style: ObsidianTypography.monoCode(
                            color: state.isSyncing
                                ? ObsidianColors.primary
                                : ObsidianColors.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}

class _Separator extends StatelessWidget {
  const _Separator();

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 1,
      height: 12,
      color: ObsidianColors.border,
    );
  }
}
