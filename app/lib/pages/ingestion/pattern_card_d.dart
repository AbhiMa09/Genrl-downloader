import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

/// Pattern Card D: Daemon Ping Reconnect
class PatternCardD extends StatelessWidget {
  const PatternCardD({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border, width: 0.5),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text('Daemon Ping Reconnect',
                    style: ObsidianTypography.titleSm(),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.errorContainer.withOpacity(0.4),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('RPC RETRY',
                    style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.error)),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            'Handshake monitor when qBittorrent WebUI reloads or unix socket restarts.',
            style: ObsidianTypography.bodySm(
                color: ObsidianColors.onSurfaceVariant),
          ),
          const Spacer(),

          // Showcase Canvas
          Container(
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Row(
                      children: [
                        Container(
                          width: 6,
                          height: 6,
                          decoration: const BoxDecoration(
                            color: ObsidianColors.tertiary,
                            shape: BoxShape.circle,
                          ),
                        ),
                        const SizedBox(width: 6),
                        Text(
                          '127.0.0.1:8080',
                          style: ObsidianTypography.monoCode(fontSize: 10),
                        ),
                      ],
                    ),
                    Text(
                      '12 ms',
                      style: ObsidianTypography.monoMetric(
                        color: ObsidianColors.tertiary,
                        fontSize: 10,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                ClipRRect(
                  borderRadius: BorderRadius.circular(4),
                  child: const LinearProgressIndicator(
                    value: 0.82,
                    backgroundColor: ObsidianColors.surfaceContainerHighest,
                    valueColor: AlwaysStoppedAnimation<Color>(
                        ObsidianColors.tertiary),
                    minHeight: 5,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 10),

          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Flexible(
                child: Text('Heartbeat: 2000ms',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
              const SizedBox(width: 4),
              Flexible(
                child: Text('TCP_NODELAY',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.tertiary, fontSize: 10),
                    overflow: TextOverflow.ellipsis),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
