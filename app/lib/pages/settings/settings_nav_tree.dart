import 'package:flutter/material.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class SettingsNavTree extends StatelessWidget {
  final int activeIndex;
  final ValueChanged<int> onSelect;
  final StateProvider state;

  const SettingsNavTree({
    super.key,
    required this.activeIndex,
    required this.onSelect,
    required this.state,
  });

  @override
  Widget build(BuildContext context) {
    final subtabs = [
      {'label': 'Accounts', 'icon': Icons.manage_accounts_rounded, 'badge': 'MAL'},
      {'label': 'qBittorrent', 'icon': Icons.sync_alt_rounded, 'badge': 'RPC'},
      {'label': 'Folders', 'icon': Icons.folder_special_rounded, 'badge': null},
      {'label': 'Downloads', 'icon': Icons.download_for_offline_rounded, 'badge': null},
      {'label': 'Rate limits', 'icon': Icons.speed_rounded, 'badge': null},
      {'label': 'Appearance', 'icon': Icons.palette_rounded, 'badge': null},
      {'label': 'Notifications', 'icon': Icons.notifications_active_rounded, 'badge': null},
      {'label': 'Backup', 'icon': Icons.save_rounded, 'badge': null},
    ];

    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
            child: Text('CONFIGURATION TREE', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
          ),
          ...subtabs.asMap().entries.map((entry) {
            final idx = entry.key;
            final item = entry.value;
            final isSelected = activeIndex == idx;
            return InkWell(
              onTap: () => onSelect(idx),
              borderRadius: BorderRadius.circular(8),
              child: Container(
                margin: const EdgeInsets.only(bottom: 2),
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                decoration: BoxDecoration(
                  color: isSelected ? ObsidianColors.surfaceContainerHigh : Colors.transparent,
                  borderRadius: BorderRadius.circular(8),
                  border: isSelected
                      ? Border.all(color: ObsidianColors.primary.withOpacity(0.3))
                      : null,
                ),
                child: Row(
                  children: [
                    Icon(
                      item['icon'] as IconData,
                      size: 16,
                      color: isSelected ? ObsidianColors.primary : ObsidianColors.outline,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        item['label'] as String,
                        style: isSelected
                            ? ObsidianTypography.bodySm(color: ObsidianColors.primary).copyWith(fontWeight: FontWeight.bold)
                            : ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
                      ),
                    ),
                    if (item['badge'] != null)
                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                        decoration: BoxDecoration(
                          color: ObsidianColors.surfaceContainer,
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(item['badge'] as String, style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
                      )
                    else if (isSelected)
                      Container(
                        width: 6,
                        height: 6,
                        decoration: const BoxDecoration(
                          shape: BoxShape.circle,
                          color: ObsidianColors.primary,
                        ),
                      ),
                  ],
                ),
              ),
            );
          }),
          const SizedBox(height: 16),
          // System Telemetry Micro-Card in Nav
          Container(
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Expanded(
                      child: Text('ENV: LINUX x86_64', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline), overflow: TextOverflow.ellipsis),
                    ),
                    const SizedBox(width: 4),
                    Text('RPC OK', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
                  ],
                ),
                const SizedBox(height: 6),
                ClipRRect(
                  borderRadius: BorderRadius.circular(2),
                  child: const LinearProgressIndicator(
                    value: 0.75,
                    backgroundColor: ObsidianColors.surfaceContainer,
                    valueColor: AlwaysStoppedAnimation<Color>(ObsidianColors.tertiary),
                    minHeight: 4,
                  ),
                ),
                const SizedBox(height: 6),
                Text('Conf: /etc/genrl/core.yaml', style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurfaceVariant)),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
