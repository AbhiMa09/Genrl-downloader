import 'package:flutter/material.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class DashboardOperationalBar extends StatelessWidget {
  final StateProvider state;
  final TextEditingController searchFilterController;

  const DashboardOperationalBar({
    super.key,
    required this.state,
    required this.searchFilterController,
  });

  @override
  Widget build(BuildContext context) {
    final status = state.status;

    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Wrap(
        alignment: WrapAlignment.spaceBetween,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: 12,
        runSpacing: 10,
        children: [
          // Search & Quick Jump Box
          ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 420, minWidth: 260),
            child: Container(
              height: 38,
              padding: const EdgeInsets.symmetric(horizontal: 10),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLowest,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: ObsidianColors.border),
              ),
              child: Row(
                children: [
                  const Icon(Icons.travel_explore_rounded, size: 18, color: ObsidianColors.outline),
                  const SizedBox(width: 8),
                  Expanded(
                    child: TextField(
                      controller: searchFilterController,
                      style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
                      decoration: InputDecoration(
                        hintText: 'Quick jump or filter anime... (Ctrl+K)',
                        hintStyle: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                        border: InputBorder.none,
                        isDense: true,
                      ),
                      onSubmitted: (val) {
                        if (val.trim().isNotEmpty) {
                          state.selectPage(4); // Search page
                        }
                      },
                    ),
                  ),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 2),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerHighest,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text('Ctrl+K', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 10)),
                  ),
                ],
              ),
            ),
          ),

          // qBit Link & Action Buttons
          Wrap(
            spacing: 8,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: 7,
                      height: 7,
                      decoration: BoxDecoration(
                        color: status.qbitConnected ? ObsidianColors.tertiary : ObsidianColors.error,
                        shape: BoxShape.circle,
                      ),
                    ),
                    const SizedBox(width: 8),
                    Text(
                      'qBittorrent ${status.qbitVersion} :: ${status.qbitConnected ? "IPC Linked" : "Offline"}',
                      style: ObsidianTypography.monoCode(color: ObsidianColors.onSurfaceVariant, fontSize: 11),
                    ),
                  ],
                ),
              ),

              OutlinedButton.icon(
                onPressed: state.isSyncing ? null : () => state.forceSyncMal(),
                icon: state.isSyncing
                    ? const SizedBox(width: 14, height: 14, child: CircularProgressIndicator(strokeWidth: 2, color: ObsidianColors.primary))
                    : const Icon(Icons.sync_rounded, size: 16, color: ObsidianColors.primary),
                label: Text('Sync MyAnimeList', style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface)),
                style: OutlinedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainer,
                  side: const BorderSide(color: ObsidianColors.border),
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
              ),

              OutlinedButton.icon(
                onPressed: state.isScanning ? null : () => state.manualDiskScan(),
                icon: state.isScanning
                    ? const SizedBox(width: 14, height: 14, child: CircularProgressIndicator(strokeWidth: 2, color: ObsidianColors.tertiary))
                    : const Icon(Icons.folder_special_rounded, size: 16, color: ObsidianColors.tertiary),
                label: Text('Manual Disk Scan', style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface)),
                style: OutlinedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainer,
                  side: const BorderSide(color: ObsidianColors.border),
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
              ),

              ElevatedButton.icon(
                onPressed: () async => await state.grabNextAvailable(),
                icon: const Icon(Icons.bolt_rounded, size: 16),
                label: Text('Grab Next Available', style: ObsidianTypography.titleSm(color: ObsidianColors.onPrimaryContainer, fontSize: 12)),
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.primaryContainer,
                  foregroundColor: ObsidianColors.onPrimaryContainer,
                  elevation: 0,
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
