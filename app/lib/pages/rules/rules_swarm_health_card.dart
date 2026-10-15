import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class RulesSwarmHealthCard extends StatelessWidget {
  final TextEditingController autoSeedController;
  final TextEditingController triageMinController;
  final TextEditingController triageMaxController;
  final bool bypassInboxStagnation;
  final ValueChanged<bool> onBypassChanged;

  const RulesSwarmHealthCard({
    super.key,
    required this.autoSeedController,
    required this.triageMinController,
    required this.triageMaxController,
    required this.bypassInboxStagnation,
    required this.onBypassChanged,
  });

  @override
  Widget build(BuildContext context) {
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
          Wrap(
            alignment: WrapAlignment.spaceBetween,
            crossAxisAlignment: WrapCrossAlignment.center,
            spacing: 8,
            runSpacing: 4,
            children: [
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(Icons.speed_rounded, size: 20, color: ObsidianColors.tertiary),
                  const SizedBox(width: 8),
                  Text('Swarm Health & Ingest Criteria', style: ObsidianTypography.titleSm()),
                ],
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.tertiary.withOpacity(0.2),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('Daemon Live', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
              ),
            ],
          ),
          const SizedBox(height: 4),
          Text(
            'Define boundary conditions for unattended automatic download versus triage routing into the user Inbox.',
            style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
          ),
          const SizedBox(height: 14),

          // Two Inputs Side-by-Side
          Row(
            children: [
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: ObsidianColors.border),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Expanded(
                            child: Text(
                              'AUTO-COMMIT TARGET',
                              style: ObsidianTypography.labelBadge(color: ObsidianColors.outline),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                          const SizedBox(width: 4),
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                            decoration: BoxDecoration(
                              color: ObsidianColors.tertiary.withOpacity(0.2),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text('Direct DL', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
                          ),
                        ],
                      ),
                      const SizedBox(height: 8),
                      Wrap(
                        crossAxisAlignment: WrapCrossAlignment.center,
                        spacing: 6,
                        children: [
                          Text('At or above:', style: ObsidianTypography.bodySm()),
                          SizedBox(
                            width: 50,
                            height: 30,
                            child: TextField(
                              controller: autoSeedController,
                              textAlign: TextAlign.center,
                              style: ObsidianTypography.monoCode(color: ObsidianColors.primary).copyWith(fontWeight: FontWeight.bold),
                              decoration: const InputDecoration(
                                border: OutlineInputBorder(),
                                contentPadding: EdgeInsets.zero,
                              ),
                            ),
                          ),
                          Text('seeders', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                        ],
                      ),
                      const SizedBox(height: 4),
                      Text('Transfers to qBittorrent immediately', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: ObsidianColors.border),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Expanded(
                            child: Text(
                              'TRIAGE ROUTE',
                              style: ObsidianTypography.labelBadge(color: ObsidianColors.outline),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                          const SizedBox(width: 4),
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                            decoration: BoxDecoration(
                              color: ObsidianColors.error.withOpacity(0.2),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text('Inbox Gate', style: ObsidianTypography.labelBadge(color: ObsidianColors.error)),
                          ),
                        ],
                      ),
                      const SizedBox(height: 8),
                      Wrap(
                        crossAxisAlignment: WrapCrossAlignment.center,
                        spacing: 4,
                        children: [
                          Text('Between:', style: ObsidianTypography.bodySm()),
                          SizedBox(
                            width: 38,
                            height: 30,
                            child: TextField(
                              controller: triageMinController,
                              textAlign: TextAlign.center,
                              style: ObsidianTypography.monoCode(),
                              decoration: const InputDecoration(border: OutlineInputBorder(), contentPadding: EdgeInsets.zero),
                            ),
                          ),
                          Text('&', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                          SizedBox(
                            width: 38,
                            height: 30,
                            child: TextField(
                              controller: triageMaxController,
                              textAlign: TextAlign.center,
                              style: ObsidianTypography.monoCode(),
                              decoration: const InputDecoration(border: OutlineInputBorder(), contentPadding: EdgeInsets.zero),
                            ),
                          ),
                          Text('seeds', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                        ],
                      ),
                      const SizedBox(height: 4),
                      Text('Requires manual one-click confirmation', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                    ],
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 14),

          // Spectrum Bar Visualization
          Wrap(
            alignment: WrapAlignment.spaceBetween,
            spacing: 8,
            runSpacing: 4,
            children: [
              Text('0 Seeds (Dropped)', style: ObsidianTypography.labelBadge(color: ObsidianColors.error)),
              Text('1 - 9 Seeds (Inbox Approval)', style: ObsidianTypography.labelBadge(color: ObsidianColors.secondary)),
              Text('10+ Seeds (Automated)', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
            ],
          ),
          const SizedBox(height: 4),
          ClipRRect(
            borderRadius: BorderRadius.circular(6),
            child: SizedBox(
              height: 10,
              child: Row(
                children: [
                  Expanded(flex: 12, child: Container(color: ObsidianColors.error)),
                  Expanded(flex: 38, child: Container(color: ObsidianColors.secondary)),
                  Expanded(flex: 50, child: Container(color: ObsidianColors.tertiary)),
                ],
              ),
            ),
          ),
          const SizedBox(height: 14),

          // Fallback switch
          Material(
            type: MaterialType.transparency,
            child: SwitchListTile(
              value: bypassInboxStagnation,
              onChanged: (v) => onBypassChanged(v),
              title: Text('Bypass Inbox Stagnation', style: ObsidianTypography.bodySm().copyWith(fontWeight: FontWeight.bold)),
              subtitle: Text('Auto-release item from Inbox to downloader if torrent is >48 hours old and no higher-seed release appears.', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
              activeColor: ObsidianColors.primary,
              contentPadding: EdgeInsets.zero,
            ),
          ),
        ],
      ),
    );
  }
}
