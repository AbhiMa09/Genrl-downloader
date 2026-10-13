import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';

class AppHeader extends StatelessWidget {
  final String pageTitle;

  const AppHeader({
    super.key,
    required this.pageTitle,
  });

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
          // Page Title & Linux Tag
          Row(
            children: [
              Text(
                pageTitle,
                style: ObsidianTypography.headlineMd(),
              ),
              const SizedBox(width: 10),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  'Arch Linux',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.outline,
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
                    style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
                    decoration: InputDecoration(
                      hintText: 'Quick Jump or Filter...',
                      hintStyle: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                      border: InputBorder.none,
                      isDense: true,
                      contentPadding: EdgeInsets.zero,
                    ),
                  ),
                ),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1.5),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    'Ctrl+K',
                    style: ObsidianTypography.labelBadge(color: ObsidianColors.outline),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: 16),

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
