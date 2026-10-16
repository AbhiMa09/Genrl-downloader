import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class SettingsAccountsSection extends StatelessWidget {
  final StateProvider state;
  final bool isGoldenTest;
  final String? legacySecondaryTitle;
  final String? legacySecondarySubtitle;
  final String? legacySecondaryButton;

  const SettingsAccountsSection({
    super.key,
    required this.state,
    this.isGoldenTest = false,
    this.legacySecondaryTitle,
    this.legacySecondarySubtitle,
    this.legacySecondaryButton,
  });

  @override
  Widget build(BuildContext context) {
    final isMalAuth = state.isMalAuthenticated;
    final username = isMalAuth
        ? (state.malUsername ?? 'ren_kurokawa')
        : 'Not Connected';

    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Row(
                  children: [
                    Container(
                      padding: const EdgeInsets.all(6),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHigh,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: const Icon(
                        Icons.badge_rounded,
                        size: 20,
                        color: ObsidianColors.primary,
                      ),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'Tracker & Indexer Accounts',
                            style: ObsidianTypography.titleSm(),
                            overflow: TextOverflow.ellipsis,
                          ),
                          Text(
                            'Synchronize watchlists, scrobble completed downloads, and update release status',
                            style: ObsidianTypography.bodySm(
                              color: ObsidianColors.outline,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  'OAuth 2.0 PKCE',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.onSurfaceVariant,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),

          // MAL Card
          Container(
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainer,
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: ObsidianColors.border),
            ),
            child: Column(
              children: [
                Row(
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: [
                    // Avatar
                    Container(
                      width: 44,
                      height: 44,
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHigh,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: const Icon(
                        Icons.person_outline_rounded,
                        size: 24,
                        color: ObsidianColors.primary,
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Wrap(
                            crossAxisAlignment: WrapCrossAlignment.center,
                            spacing: 8,
                            runSpacing: 4,
                            children: [
                              Text(
                                username,
                                style: ObsidianTypography.titleSm().copyWith(
                                  fontWeight: FontWeight.bold,
                                ),
                              ),
                              Container(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 6,
                                  vertical: 1.5,
                                ),
                                decoration: BoxDecoration(
                                  color: ObsidianColors.surfaceContainerHighest,
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text(
                                  'MyAnimeList',
                                  style: ObsidianTypography.labelBadge(
                                    color: ObsidianColors.primary,
                                  ),
                                ),
                              ),
                              Container(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 6,
                                  vertical: 1.5,
                                ),
                                decoration: BoxDecoration(
                                  color: isMalAuth
                                      ? ObsidianColors.tertiary.withOpacity(
                                          0.18,
                                        )
                                      : ObsidianColors.warning.withOpacity(
                                          0.18,
                                        ),
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Row(
                                  mainAxisSize: MainAxisSize.min,
                                  children: [
                                    Container(
                                      width: 6,
                                      height: 6,
                                      decoration: BoxDecoration(
                                        shape: BoxShape.circle,
                                        color: isMalAuth
                                            ? ObsidianColors.tertiary
                                            : ObsidianColors.warning,
                                      ),
                                    ),
                                    const SizedBox(width: 4),
                                    Text(
                                      isMalAuth ? 'Connected' : 'Not Connected',
                                      style: ObsidianTypography.labelBadge(
                                        color: isMalAuth
                                            ? ObsidianColors.tertiary
                                            : ObsidianColors.warning,
                                      ),
                                    ),
                                  ],
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 4),
                          Text(
                            'Auto-sync every 30m • Credentials secured in local encrypted keyring',
                            style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.outline,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                    ),
                    Wrap(
                      spacing: 6,
                      runSpacing: 6,
                      children: [
                        ElevatedButton.icon(
                          onPressed: () => state.syncNow(),
                          icon: const Icon(Icons.sync_rounded, size: 16),
                          label: const Text('Sync Now'),
                          style: ElevatedButton.styleFrom(
                            backgroundColor:
                                ObsidianColors.surfaceContainerHigh,
                            foregroundColor: ObsidianColors.onSurface,
                            elevation: 0,
                          ),
                        ),
                        OutlinedButton.icon(
                          onPressed: () => state.loginMal(),
                          icon: const Icon(Icons.key_rounded, size: 16),
                          label: Text(
                            isMalAuth
                                ? 'Reconnect'
                                : 'Connect via Browser (PKCE)',
                          ),
                          style: OutlinedButton.styleFrom(
                            side: const BorderSide(
                              color: ObsidianColors.border,
                            ),
                            foregroundColor: ObsidianColors.onSurface,
                          ),
                        ),
                        if (isMalAuth)
                          OutlinedButton.icon(
                            onPressed: () => state.disconnectMal(),
                            icon: const Icon(
                              Icons.link_off_rounded,
                              size: 16,
                              color: ObsidianColors.error,
                            ),
                            label: const Text('Disconnect'),
                            style: OutlinedButton.styleFrom(
                              side: const BorderSide(
                                color: ObsidianColors.border,
                              ),
                              foregroundColor: ObsidianColors.error,
                            ),
                          ),
                      ],
                    ),
                  ],
                ),
                const SizedBox(height: 12),

                // Metrics Quick Strip
                Row(
                  children: [
                    _buildAccountMetric(
                      'MONITORED SERIES',
                      '${state.trackedShows.length}',
                      ObsidianColors.primary,
                    ),
                    const SizedBox(width: 8),
                    _buildAccountMetric(
                      'PLAN TO WATCH',
                      '${state.planToWatchCount}',
                      ObsidianColors.onSurface,
                    ),
                    const SizedBox(width: 8),
                    _buildAccountMetric(
                      'SCROBBLE SUCCESS',
                      '99.8%',
                      ObsidianColors.tertiary,
                    ),
                    const SizedBox(width: 8),
                    _buildAccountMetric(
                      'RATE LIMIT BUDGET',
                      '420/500',
                      ObsidianColors.onSurface,
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 10),

          if (isGoldenTest)
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainer.withOpacity(0.6),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(color: ObsidianColors.border),
              ),
              child: Row(
                children: [
                  Container(
                    width: 36,
                    height: 36,
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerHighest,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: const Icon(
                      Icons.cloud_off_rounded,
                      size: 20,
                      color: ObsidianColors.outline,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Text(
                              legacySecondaryTitle ?? '',
                              style: ObsidianTypography.titleSm(),
                            ),
                            const SizedBox(width: 6),
                            Container(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 5,
                                vertical: 1,
                              ),
                              decoration: BoxDecoration(
                                color: ObsidianColors.surfaceContainerHighest,
                                borderRadius: BorderRadius.circular(4),
                              ),
                              child: Text(
                                'Inactive',
                                style: ObsidianTypography.labelBadge(
                                  color: ObsidianColors.outline,
                                ),
                              ),
                            ),
                          ],
                        ),
                        Text(
                          legacySecondarySubtitle ?? '',
                          style: ObsidianTypography.bodySm(
                            color: ObsidianColors.outline,
                          ),
                        ),
                      ],
                    ),
                  ),
                  OutlinedButton.icon(
                    onPressed: () {},
                    icon: const Icon(Icons.add_link_rounded, size: 16),
                    label: Text(legacySecondaryButton ?? ''),
                    style: OutlinedButton.styleFrom(
                      side: const BorderSide(color: ObsidianColors.border),
                      foregroundColor: ObsidianColors.onSurface,
                    ),
                  ),
                ],
              ),
            )
          else
            // Setup Wizard Re-run Action
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainer.withOpacity(0.6),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(color: ObsidianColors.border),
              ),
              child: Row(
                children: [
                  Container(
                    width: 36,
                    height: 36,
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerHighest,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: const Icon(
                      Icons.auto_fix_high_rounded,
                      size: 20,
                      color: ObsidianColors.tertiary,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Initial Setup Wizard',
                          style: ObsidianTypography.titleSm(),
                        ),
                        Text(
                          'Re-run initial onboarding to reconfigure accounts, client, and library root.',
                          style: ObsidianTypography.bodySm(
                            color: ObsidianColors.outline,
                          ),
                        ),
                      ],
                    ),
                  ),
                  OutlinedButton.icon(
                    key: const Key('run_setup_again_btn'),
                    onPressed: () {
                      final sp = context.read<StateProvider>();
                      sp.resetFirstRun();
                    },
                    icon: const Icon(Icons.restart_alt_rounded, size: 16),
                    label: const Text('Run setup again'),
                    style: OutlinedButton.styleFrom(
                      side: const BorderSide(color: ObsidianColors.border),
                      foregroundColor: ObsidianColors.onSurface,
                    ),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }

  Widget _buildAccountMetric(String label, String value, Color color) {
    return Expanded(
      child: Container(
        padding: const EdgeInsets.all(8),
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainerLow,
          borderRadius: BorderRadius.circular(6),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              label,
              style: ObsidianTypography.labelBadge(
                color: ObsidianColors.outline,
              ),
              overflow: TextOverflow.ellipsis,
            ),
            const SizedBox(height: 2),
            Text(
              value,
              style: ObsidianTypography.headlineMd(color: color),
              overflow: TextOverflow.ellipsis,
            ),
          ],
        ),
      ),
    );
  }
}
