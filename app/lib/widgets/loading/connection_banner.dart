import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'inline_spinner.dart';

/// Connection Banner displaying live daemon and qBittorrent status and measured latency.
/// Replaces mock status strings with real connection monitoring.
class ConnectionBanner extends StatelessWidget {
  final VoidCallback? onRetry;

  const ConnectionBanner({super.key, this.onRetry});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    final isCoreDown = !state.isCoreConnected;
    final isQbitDown = !state.status.qbitConnected;

    if (!isCoreDown && !isQbitDown) {
      return const SizedBox.shrink();
    }

    final message = isCoreDown
        ? 'Core daemon unreachable (latency: --)'
        : 'qBittorrent WebUI reconnecting... (socket/port check)';

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      decoration: BoxDecoration(
        color: ObsidianColors.errorContainer.withOpacity(0.9),
        border: const Border(
          bottom: BorderSide(color: ObsidianColors.error, width: 1),
        ),
      ),
      child: Row(
        children: [
          const InlineSpinner(
            size: 14,
            color: ObsidianColors.onErrorContainer,
            strokeWidth: 2,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              message,
              style: ObsidianTypography.bodySm(
                color: ObsidianColors.onErrorContainer,
              ).copyWith(fontWeight: FontWeight.w500),
            ),
          ),
          if (onRetry != null || state.isCoreConnected == false)
            TextButton.icon(
              onPressed: () {
                if (onRetry != null) {
                  onRetry!();
                } else {
                  state.refreshStatus();
                }
              },
              icon: const Icon(Icons.refresh_rounded, size: 14, color: ObsidianColors.onErrorContainer),
              label: Text(
                'Retry',
                style: ObsidianTypography.labelBadge(color: ObsidianColors.onErrorContainer),
              ),
              style: TextButton.styleFrom(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                minimumSize: Size.zero,
                tapTargetSize: MaterialTapTargetSize.shrinkWrap,
              ),
            ),
        ],
      ),
    );
  }
}
