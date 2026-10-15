import 'package:flutter/material.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../widgets/loading/inline_spinner.dart';

/// Compact Ingestion Strip shown on the Dashboard during active syncs or background ingestion.
class DashboardIngestionStrip extends StatelessWidget {
  final StateProvider state;

  const DashboardIngestionStrip({super.key, required this.state});

  @override
  Widget build(BuildContext context) {
    if (!state.isIngesting && !state.isSyncing) {
      return const SizedBox.shrink();
    }

    final stageName = state.isIngesting ? state.ingestionStage.name.toUpperCase() : 'SYNCING';
    final progress = state.ingestionTotalCount > 0
        ? (state.ingestionProcessedCount / state.ingestionTotalCount).clamp(0.0, 1.0)
        : 0.0;

    return Container(
      margin: const EdgeInsets.only(bottom: 16),
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: ObsidianColors.secondary.withOpacity(0.4),
          width: 0.8,
        ),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Row(
            children: [
              const InlineSpinner(size: 14, color: ObsidianColors.secondary),
              const SizedBox(width: 10),
              Expanded(
                child: Row(
                  children: [
                    Text(
                      'INGESTION ACTIVE [$stageName]: ',
                      style: ObsidianTypography.monoCode(color: ObsidianColors.secondary).copyWith(fontWeight: FontWeight.bold),
                    ),
                    Flexible(
                      child: Text(
                        state.isIngesting
                            ? '${state.ingestionProcessedCount} / ${state.ingestionTotalCount} items processed'
                            : 'Synchronizing library manifests with core...',
                        style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ],
                ),
              ),
              if (state.ingestionEtaSeconds != null && state.ingestionEtaSeconds! > 0)
                Text(
                  '~${state.ingestionEtaSeconds}s remaining',
                  style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
                ),
              const SizedBox(width: 12),
              InkWell(
                onTap: () => state.selectPage(5),
                child: Text(
                  'View Telemetry',
                  style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary),
                ),
              ),
            ],
          ),
          if (state.isIngesting) ...[
            const SizedBox(height: 8),
            ClipRRect(
              borderRadius: BorderRadius.circular(2),
              child: LinearProgressIndicator(
                value: progress,
                minHeight: 3,
                backgroundColor: ObsidianColors.surfaceContainerLowest,
                valueColor: const AlwaysStoppedAnimation<Color>(ObsidianColors.primary),
              ),
            ),
          ],
        ],
      ),
    );
  }
}
