import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';

class NavRail extends StatelessWidget {
  const NavRail({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    return Container(
      width: 208,
      decoration: const BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        border: Border(
          right: BorderSide(color: ObsidianColors.border, width: 1),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // App Header & Branding
          Container(
            height: 56,
            padding: const EdgeInsets.symmetric(horizontal: 16),
            alignment: Alignment.centerLeft,
            child: Row(
              children: [
                Container(
                  width: 28,
                  height: 28,
                  decoration: BoxDecoration(
                    color: ObsidianColors.primaryContainer,
                    borderRadius: BorderRadius.circular(6),
                    boxShadow: [
                      BoxShadow(
                        color: ObsidianColors.primary.withOpacity(0.3),
                        blurRadius: 8,
                        spreadRadius: 1,
                      ),
                    ],
                  ),
                  child: const Icon(
                    Icons.bolt_rounded,
                    size: 18,
                    color: ObsidianColors.onPrimaryContainer,
                  ),
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Genrl',
                        style: ObsidianTypography.titleSm(),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      Text(
                        'v2.4.1 Linux x86_64',
                        style: ObsidianTypography.monoCode(
                          color: ObsidianColors.outline,
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const Divider(height: 1, color: ObsidianColors.border),

          // Navigation Links
          Expanded(
            child: ListView(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 12),
              children: [
                _NavItem(
                  index: 0,
                  label: 'Dashboard',
                  icon: Icons.space_dashboard_outlined,
                  selectedIcon: Icons.space_dashboard,
                  isSelected: state.selectedPageIndex == 0,
                  onTap: () => state.selectPage(0),
                ),
                _NavItem(
                  index: 1,
                  label: 'Inbox',
                  icon: Icons.inbox_outlined,
                  selectedIcon: Icons.inbox,
                  badge: state.inboxCount > 0 ? '${state.inboxCount}' : null,
                  badgeColor: ObsidianColors.error,
                  isSelected: state.selectedPageIndex == 1,
                  onTap: () => state.selectPage(1),
                ),
                _NavItem(
                  index: 2,
                  label: 'Queue',
                  icon: Icons.downloading_outlined,
                  selectedIcon: Icons.downloading,
                  badge: state.activeQueueCount > 0
                      ? '${state.activeQueueCount}'
                      : null,
                  badgeColor: ObsidianColors.tertiary,
                  isSelected: state.selectedPageIndex == 2,
                  onTap: () => state.selectPage(2),
                ),
                _NavItem(
                  index: 8,
                  label: 'Ingestion Stage',
                  icon: Icons.radar_outlined,
                  selectedIcon: Icons.radar_rounded,
                  badge: 'LIVE',
                  badgeColor: ObsidianColors.secondary,
                  isSelected: state.selectedPageIndex == 8,
                  onTap: () => state.selectPage(8),
                ),
                _NavItem(
                  index: 3,
                  label: 'Library',
                  icon: Icons.video_library_outlined,
                  selectedIcon: Icons.video_library,
                  isSelected: state.selectedPageIndex == 3,
                  onTap: () => state.selectPage(3),
                ),
                _NavItem(
                  index: 4,
                  label: 'Search',
                  icon: Icons.search_rounded,
                  selectedIcon: Icons.search,
                  isSelected: state.selectedPageIndex == 4,
                  onTap: () => state.selectPage(4),
                ),
                _NavItem(
                  index: 5,
                  label: 'Activity',
                  icon: Icons.monitor_heart_outlined,
                  selectedIcon: Icons.monitor_heart,
                  isSelected: state.selectedPageIndex == 5,
                  onTap: () => state.selectPage(5),
                ),
                _NavItem(
                  index: 6,
                  label: 'Rules',
                  icon: Icons.rule_folder_outlined,
                  selectedIcon: Icons.rule_folder,
                  isSelected: state.selectedPageIndex == 6,
                  onTap: () => state.selectPage(6),
                ),
                _NavItem(
                  index: 7,
                  label: 'Settings',
                  icon: Icons.settings_outlined,
                  selectedIcon: Icons.settings,
                  isSelected: state.selectedPageIndex == 7,
                  onTap: () => state.selectPage(7),
                ),
              ],
            ),
          ),

          // Bottom Daemon Health Indicator
          Padding(
            padding: const EdgeInsets.all(10),
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainer,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: ObsidianColors.border, width: 0.5),
              ),
              child: Row(
                children: [
                  Container(
                    width: 7,
                    height: 7,
                    decoration: BoxDecoration(
                      color: state.status.daemonConnected
                          ? ObsidianColors.tertiary
                          : ObsidianColors.error,
                      shape: BoxShape.circle,
                      boxShadow: [
                        BoxShadow(
                          color: (state.status.daemonConnected
                                  ? ObsidianColors.tertiary
                                  : ObsidianColors.error)
                              .withOpacity(0.5),
                          blurRadius: 4,
                          spreadRadius: 1,
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      state.status.daemonConnected ? 'Daemon OK' : 'Disconnected',
                      style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline,
                      ),
                    ),
                  ),
                  Text(
                    state.status.daemonPort,
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _NavItem extends StatelessWidget {
  final int index;
  final String label;
  final IconData icon;
  final IconData selectedIcon;
  final bool isSelected;
  final String? badge;
  final Color badgeColor;
  final VoidCallback onTap;

  const _NavItem({
    required this.index,
    required this.label,
    required this.icon,
    required this.selectedIcon,
    required this.isSelected,
    this.badge,
    this.badgeColor = ObsidianColors.tertiary,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 2),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        hoverColor: ObsidianColors.surfaceContainer,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 150),
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
          decoration: BoxDecoration(
            color: isSelected
                ? ObsidianColors.surfaceContainerHigh
                : Colors.transparent,
            borderRadius: BorderRadius.circular(8),
            border: isSelected
                ? Border.all(
                    color: ObsidianColors.primary.withOpacity(0.2), width: 1)
                : null,
          ),
          child: Row(
            children: [
              Icon(
                isSelected ? selectedIcon : icon,
                size: 18,
                color: isSelected
                    ? ObsidianColors.primary
                    : ObsidianColors.onSurfaceVariant,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  label,
                  style: isSelected
                      ? ObsidianTypography.bodySm(color: ObsidianColors.primary)
                          .copyWith(fontWeight: FontWeight.w600)
                      : ObsidianTypography.bodySm(
                          color: ObsidianColors.onSurfaceVariant),
                ),
              ),
              if (badge != null)
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
                  decoration: BoxDecoration(
                    color: badgeColor.withOpacity(0.18),
                    borderRadius: BorderRadius.circular(4),
                    border: Border.all(
                      color: badgeColor.withOpacity(0.35),
                      width: 1,
                    ),
                  ),
                  child: Text(
                    badge!,
                    style: ObsidianTypography.labelBadge(color: badgeColor),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}
