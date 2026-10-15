import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'ingestion_meta_chips.dart';

class IngestionContextStrip extends StatelessWidget {
  const IngestionContextStrip({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border, width: 0.5),
      ),
      child: Wrap(
        alignment: WrapAlignment.spaceBetween,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: 16,
        runSpacing: 10,
        children: [
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                width: 10,
                height: 10,
                decoration: const BoxDecoration(
                  color: ObsidianColors.tertiary,
                  shape: BoxShape.circle,
                ),
              ),
              const SizedBox(width: 12),
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        'Syncing & Ingestion Daemon',
                        style: ObsidianTypography.titleSm(),
                      ),
                      const SizedBox(width: 8),
                      Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 6, vertical: 2),
                        decoration: BoxDecoration(
                          color: ObsidianColors.tertiaryContainer
                              .withOpacity(0.3),
                          borderRadius: BorderRadius.circular(4),
                          border: Border.all(
                            color: ObsidianColors.tertiary
                                .withOpacity(0.3),
                            width: 0.5,
                          ),
                        ),
                        child: Text(
                          'RPC ACTIVE',
                          style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.tertiary,
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 2),
                  Text(
                    'Session PID: 40921 • IPC: /run/genrl/daemon.sock',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.outline,
                    ),
                  ),
                ],
              ),
            ],
          ),
          Wrap(
            spacing: 8,
            runSpacing: 6,
            children: [
              const IngestionMetaChip(text: 'Worker Threads: 08'),
              const IngestionMetaChip(text: 'qBt 4.6.2'),
              Container(
                padding: const EdgeInsets.symmetric(
                    horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: ObsidianColors.secondaryContainer
                      .withOpacity(0.4),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  'V2.4.1-STABLE',
                  style: ObsidianTypography.monoCode(
                    color: ObsidianColors.secondary,
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
