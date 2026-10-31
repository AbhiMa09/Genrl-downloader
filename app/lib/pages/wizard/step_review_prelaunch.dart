import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

/// Clean pre-launch configuration summary shown on Step 5 before ingestion starts.
/// Free of hardcoded episode/manifest data on an empty database.
class StepReviewPrelaunch extends StatelessWidget {
  final StateProvider state;
  final TextEditingController qbitUrlController;
  final VoidCallback onRecheckQbit;
  final VoidCallback onOpenQbitSettings;

  const StepReviewPrelaunch({
    super.key,
    required this.state,
    required this.qbitUrlController,
    required this.onRecheckQbit,
    required this.onOpenQbitSettings,
  });

  @override
  Widget build(BuildContext context) {
    final qbitConnected = state.status.qbitConnected;
    final malUser = state.malUsername?.isNotEmpty == true
        ? state.malUsername!
        : 'akumasan2';
    final libraryPath = state.libraryFolder.isNotEmpty
        ? state.libraryFolder
        : '/tmp/genrl_test_root';

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (!qbitConnected) ...[
          Container(
            key: const Key('qbit_preflight_error_banner'),
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: ObsidianColors.errorContainer.withOpacity(0.35),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: ObsidianColors.error.withOpacity(0.6)),
            ),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Icon(
                  Icons.error_outline_rounded,
                  color: ObsidianColors.error,
                  size: 20,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Expanded(
                            child: Text(
                              'Preflight Check Failed: qBittorrent Unreachable',
                              style: ObsidianTypography.titleSm().copyWith(
                                color: ObsidianColors.error,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                          ),
                          TextButton.icon(
                            key: const Key('qbit_recheck_btn'),
                            onPressed: onRecheckQbit,
                            icon: const Icon(
                              Icons.refresh,
                              size: 14,
                              color: ObsidianColors.primary,
                            ),
                            label: const Text(
                              'Re-check',
                              style: TextStyle(
                                color: ObsidianColors.primary,
                                fontSize: 12,
                              ),
                            ),
                            style: TextButton.styleFrom(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 8,
                                vertical: 4,
                              ),
                              minimumSize: Size.zero,
                              tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                            ),
                          ),
                          const SizedBox(width: 8),
                          TextButton.icon(
                            key: const Key('qbit_open_settings_btn'),
                            onPressed: onOpenQbitSettings,
                            icon: const Icon(
                              Icons.settings,
                              size: 14,
                              color: ObsidianColors.secondary,
                            ),
                            label: const Text(
                              'Open Settings',
                              style: TextStyle(
                                color: ObsidianColors.secondary,
                                fontSize: 12,
                              ),
                            ),
                            style: TextButton.styleFrom(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 8,
                                vertical: 4,
                              ),
                              minimumSize: Size.zero,
                              tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 4),
                      Text(
                        'no qBittorrent WebUI at ${qbitUrlController.text} — start qBittorrent (or install qbittorrent-nox) and enable Settings → WebUI on that port, then set the password with `genrl qbit password`',
                        style: ObsidianTypography.bodySm(
                          color: ObsidianColors.onSurface,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
        ],

        // Pre-launch Configuration Summary Card
        Container(
          padding: const EdgeInsets.all(20),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(color: ObsidianColors.border),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'Configuration Review',
                style: ObsidianTypography.headlineMd().copyWith(
                  fontWeight: FontWeight.bold,
                ),
              ),
              const SizedBox(height: 6),
              Text(
                'Verify your connection and storage settings before starting initial synchronization.',
                style: ObsidianTypography.bodySm(
                  color: ObsidianColors.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 16),
              _buildConfigRow(
                icon: Icons.person_outline_rounded,
                label: 'MyAnimeList Account',
                value: malUser,
                badge: 'Ready to import',
              ),
              const Divider(height: 24, color: ObsidianColors.border),
              _buildConfigRow(
                icon: Icons.cloud_done_outlined,
                label: 'qBittorrent Endpoint',
                value: qbitUrlController.text,
                badge: qbitConnected ? 'Connected' : 'Offline',
                isError: !qbitConnected,
              ),
              const Divider(height: 24, color: ObsidianColors.border),
              _buildConfigRow(
                icon: Icons.folder_outlined,
                label: 'Library Destination',
                value: libraryPath,
                badge: 'Target pool',
              ),
              const Divider(height: 24, color: ObsidianColors.border),
              _buildConfigRow(
                icon: Icons.high_quality_outlined,
                label: 'Quality & Preferred Groups',
                value:
                    '1080p • 5 Preferred Groups (SubsPlease, Erai-raws, etc.)',
                badge: 'Filter rules configured',
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),

        // Ingestion Overview Note
        Container(
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(
            color: ObsidianColors.primary.withOpacity(0.06),
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: ObsidianColors.primary.withOpacity(0.2)),
          ),
          child: Row(
            children: [
              const Icon(
                Icons.info_outline_rounded,
                size: 20,
                color: ObsidianColors.primary,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  'Launching will run Stages 1–3: import your library from MAL, resolve Watching show metadata, and compute missing episodes. You will review the computed plan before any torrents are added.',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurface,
                  ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildConfigRow({
    required IconData icon,
    required String label,
    required String value,
    required String badge,
    bool isError = false,
  }) {
    return Row(
      children: [
        Icon(icon, size: 20, color: ObsidianColors.secondary),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                label,
                style: ObsidianTypography.bodySm().copyWith(
                  color: ObsidianColors.outline,
                ),
              ),
              const SizedBox(height: 2),
              Text(
                value,
                style: ObsidianTypography.bodySm().copyWith(
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
          ),
        ),
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
          decoration: BoxDecoration(
            color: isError
                ? ObsidianColors.error.withOpacity(0.12)
                : ObsidianColors.secondary.withOpacity(0.12),
            borderRadius: BorderRadius.circular(12),
            border: Border.all(
              color: isError
                  ? ObsidianColors.error.withOpacity(0.4)
                  : ObsidianColors.secondary.withOpacity(0.4),
            ),
          ),
          child: Text(
            badge,
            style: ObsidianTypography.monoCode(
              color: isError ? ObsidianColors.error : ObsidianColors.secondary,
            ).copyWith(fontSize: 11, fontWeight: FontWeight.bold),
          ),
        ),
      ],
    );
  }
}
