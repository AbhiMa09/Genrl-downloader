import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import 'tray_notifications_overlay.dart';

class AppHeader extends StatelessWidget {
  final String pageTitle;

  const AppHeader({super.key, required this.pageTitle});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    return Container(
      height: 56,
      padding: const EdgeInsets.symmetric(horizontal: 24),
      decoration: const BoxDecoration(
        color: ObsidianColors.surface,
        border: Border(
          bottom: BorderSide(color: ObsidianColors.border, width: 1),
        ),
      ),
      child: Row(
        children: [
          // Page Title & Linux Tag & Core Connection Indicator
          Row(
            children: [
              Text(pageTitle, style: ObsidianTypography.headlineMd()),
              const SizedBox(width: 10),
              // Core Connection Indicator Pill
              InkWell(
                onTap: () => state.refreshAll(),
                borderRadius: BorderRadius.circular(4),
                child: Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 3,
                  ),
                  decoration: BoxDecoration(
                    color: state.isCoreConnected
                        ? ObsidianColors.success.withOpacity(0.15)
                        : ObsidianColors.error.withOpacity(0.2),
                    borderRadius: BorderRadius.circular(4),
                    border: Border.all(
                      color: state.isCoreConnected
                          ? ObsidianColors.success.withOpacity(0.5)
                          : ObsidianColors.error,
                      width: 1,
                    ),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Container(
                        width: 7,
                        height: 7,
                        decoration: BoxDecoration(
                          shape: BoxShape.circle,
                          color: state.isCoreConnected
                              ? ObsidianColors.success
                              : ObsidianColors.error,
                        ),
                      ),
                      const SizedBox(width: 6),
                      Text(
                        state.isCoreConnected
                            ? 'Core Online'
                            : 'Core Disconnected (Retry)',
                        style: ObsidianTypography.labelBadge(
                          color: state.isCoreConnected
                              ? ObsidianColors.success
                              : ObsidianColors.error,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
          const Spacer(),

          // Search / Quick Jump Input
          Container(
            width: 280,
            height: 34,
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: ObsidianColors.border, width: 1),
            ),
            padding: const EdgeInsets.symmetric(horizontal: 10),
            child: Row(
              children: [
                const Icon(
                  Icons.search_rounded,
                  size: 16,
                  color: ObsidianColors.outline,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: TextField(
                    onChanged: (val) => state.setSearchFilter(val),
                    style: ObsidianTypography.bodySm(
                      color: ObsidianColors.onSurface,
                    ),
                    decoration: InputDecoration(
                      hintText: 'Quick Jump or Filter...',
                      hintStyle: ObsidianTypography.bodySm(
                        color: ObsidianColors.outline,
                      ),
                      border: InputBorder.none,
                      isDense: true,
                      contentPadding: EdgeInsets.zero,
                    ),
                  ),
                ),
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 5,
                    vertical: 1.5,
                  ),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    'Ctrl+K',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: 16),

          const SizedBox(width: 12),

          // Tray & Notification Toasts Overlay Button
          IconButton(
            onPressed: () {
              showDialog<void>(
                context: context,
                barrierColor: Colors.transparent,
                builder: (ctx) => SystemTrayNotificationOverlay(
                  onClose: () => Navigator.pop(ctx),
                ),
              );
            },
            icon: Stack(
              clipBehavior: Clip.none,
              children: [
                const Icon(
                  Icons.notifications_active_rounded,
                  size: 20,
                  color: ObsidianColors.primary,
                ),
                Positioned(
                  right: -2,
                  top: -2,
                  child: Container(
                    width: 7,
                    height: 7,
                    decoration: const BoxDecoration(
                      shape: BoxShape.circle,
                      color: ObsidianColors.error,
                    ),
                  ),
                ),
              ],
            ),
            tooltip: 'System Tray & Notifications',
          ),
          const SizedBox(width: 8),

          // User Profile / Status Avatar
          Container(
            width: 32,
            height: 32,
            decoration: const BoxDecoration(
              color: ObsidianColors.primary,
              shape: BoxShape.circle,
            ),
            child: const Icon(
              Icons.person_rounded,
              size: 18,
              color: ObsidianColors.onPrimary,
            ),
          ),
        ],
      ),
    );
  }
}
