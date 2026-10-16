import 'package:flutter/material.dart';

import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class ShowDetailSynopsis extends StatelessWidget {
  final StateProvider state;
  final LibraryShow show;

  const ShowDetailSynopsis({
    super.key,
    required this.state,
    required this.show,
  });

  @override
  Widget build(BuildContext context) {
    final status = show.metadataStatus;
    final isFetching =
        status == 'fetching' || show.synopsis == '[Synopsis loading...]';
    final isPendingOrFailed =
        status == 'pending' ||
        status == 'failed' ||
        show.synopsis.startsWith('[Synopsis unavailable');

    if (isFetching) {
      return Container(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainerLow,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: ObsidianColors.border),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            const SizedBox(
              width: 14,
              height: 14,
              child: CircularProgressIndicator(
                strokeWidth: 2,
                color: ObsidianColors.primary,
              ),
            ),
            const SizedBox(width: 10),
            Text(
              'Synopsis loading...',
              style: ObsidianTypography.bodyMd(color: ObsidianColors.outline)
                  .copyWith(fontStyle: FontStyle.italic),
            ),
          ],
        ),
      );
    }

    if (isPendingOrFailed) {
      final errorMsg =
          show.metadataError ?? 'upstream rate limit or network unreachable';
      String retryNotice = 'Could not load, retrying';
      if (show.metadataNextRetryAt != null) {
        final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
        final diff = show.metadataNextRetryAt! - now;
        if (diff > 0) {
          retryNotice = 'Could not load, retrying in ${diff}s';
        }
      }

      return Container(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainerLow,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: ObsidianColors.errorContainer.withOpacity(0.5),
          ),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            const Icon(
              Icons.info_outline,
              size: 16,
              color: ObsidianColors.warning,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                '$retryNotice ($errorMsg)',
                style: ObsidianTypography.bodySm(
                  color: ObsidianColors.onSurfaceVariant,
                ),
              ),
            ),
            const SizedBox(width: 12),
            FilledButton.tonal(
              onPressed: () =>
                  state.refreshShowMetadata(show.malId, force: true),
              style: FilledButton.styleFrom(
                backgroundColor: ObsidianColors.surfaceContainerHigh,
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 8,
                ),
                minimumSize: Size.zero,
                tapTargetSize: MaterialTapTargetSize.shrinkWrap,
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(
                    Icons.refresh_rounded,
                    size: 14,
                    color: ObsidianColors.primary,
                  ),
                  const SizedBox(width: 6),
                  Text(
                    'Retry',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.primary,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      );
    }

    if (show.synopsis.isNotEmpty && !show.synopsis.startsWith('[')) {
      return Text(
        show.synopsis,
        style: ObsidianTypography.bodyMd(color: ObsidianColors.onSurfaceVariant)
            .copyWith(height: 1.55),
      );
    }

    return Text(
      'No synopsis available for this show.',
      style: ObsidianTypography.bodyMd(color: ObsidianColors.outline)
          .copyWith(fontStyle: FontStyle.italic),
    );
  }
}
