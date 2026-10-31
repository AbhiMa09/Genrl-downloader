import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'wizard_daily_cap_warning.dart';
import 'wizard_headline_card.dart';
import 'wizard_series_manifest.dart';

/// Screen presented after Stage 3 (Plan Queue) completes, allowing user to review
/// the computed candidates, estimated storage, manifest, and toggle cap bypass before
/// committing Stage 4 (Create Jobs).
class WizardPlanConfirmation extends StatelessWidget {
  final StateProvider state;
  final bool bypassDailyCap;
  final ValueChanged<bool> onToggleBypass;
  final VoidCallback onConfirm;
  final VoidCallback onBack;

  const WizardPlanConfirmation({
    super.key,
    required this.state,
    required this.bypassDailyCap,
    required this.onToggleBypass,
    required this.onConfirm,
    required this.onBack,
  });

  @override
  Widget build(BuildContext context) {
    final watchingShows = state.shows.where(
      (s) => s.status.toLowerCase() == 'watching',
    );
    final watchingCount = watchingShows.length;
    final resolvedWatching = watchingShows
        .where((s) => s.metadataResolved)
        .length;
    final totalShows = state.shows.length;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Header Banner
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          decoration: BoxDecoration(
            color: ObsidianColors.tertiary.withOpacity(0.08),
            borderRadius: BorderRadius.circular(10),
            border: Border.all(color: ObsidianColors.tertiary.withOpacity(0.3)),
          ),
          child: Row(
            children: [
              const Icon(
                Icons.check_circle_outline_rounded,
                color: ObsidianColors.tertiary,
                size: 22,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'Plan Ready: ${WizardSeriesManifest.seasonCount(state)} Seasons • ${state.plannedEpisodesCount} Episodes Planned',
                      style: ObsidianTypography.titleSm().copyWith(
                        color: ObsidianColors.tertiary,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      'Review every watching season and its planned episodes before creating download jobs.',
                      style: ObsidianTypography.bodySm().copyWith(
                        color: ObsidianColors.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),

        // Headline Metrics: Shows, Episodes, Est Storage, Metadata
        WizardHeadlineCard(
          plannedEpisodes: state.plannedEpisodesCount,
          plannedShows: state.plannedShowsCount,
          estimatedStorageGb: state.estimatedStorageGb,
          metadataText:
              '$resolvedWatching/$watchingCount watching, $totalShows total',
        ),
        const SizedBox(height: 16),

        // Daily Torrent Cap Warning & Bypass Toggle
        WizardDailyCapWarning(
          bypassDailyCap: bypassDailyCap,
          onToggleBypass: onToggleBypass,
          dailyCap: state.firstSyncCap > 0 ? state.firstSyncCap : 25,
          plannedEpisodes: state.plannedEpisodesCount,
        ),
        const SizedBox(height: 16),

        // Series Manifest with real posters and episode candidates
        WizardSeriesManifest(state: state),
        const SizedBox(height: 20),

        // Action Buttons Row
        Row(
          mainAxisAlignment: MainAxisAlignment.end,
          children: [
            OutlinedButton(
              key: const Key('plan_confirmation_back_btn'),
              onPressed: onBack,
              style: OutlinedButton.styleFrom(
                side: const BorderSide(color: ObsidianColors.border),
                foregroundColor: ObsidianColors.onSurfaceVariant,
                padding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 12,
                ),
              ),
              child: const Text('Back to Review'),
            ),
            const SizedBox(width: 12),
            ElevatedButton.icon(
              key: const Key('confirm_plan_btn'),
              onPressed: onConfirm,
              icon: const Icon(Icons.download_done_rounded, size: 18),
              label: Text(
                'Confirm & Create ${state.plannedEpisodesCount} Download Jobs',
              ),
              style: ElevatedButton.styleFrom(
                backgroundColor: ObsidianColors.primary,
                foregroundColor: ObsidianColors.onPrimary,
                padding: const EdgeInsets.symmetric(
                  horizontal: 20,
                  vertical: 12,
                ),
              ),
            ),
          ],
        ),
      ],
    );
  }
}
