import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'dialogs.dart';

class ShowDetailSubHeader extends StatelessWidget {
  final LibraryShow show;
  final bool isMovie;

  const ShowDetailSubHeader({
    super.key,
    required this.show,
    required this.isMovie,
  });

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    return Container(
      width: double.infinity,
      color: ObsidianColors.surfaceContainerLow,
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: [
            // Back button
            InkWell(
              onTap: () => state.closeShowDetail(),
              borderRadius: BorderRadius.circular(8),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Container(
                    width: 32,
                    height: 32,
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: const Icon(
                      Icons.arrow_back_rounded,
                      size: 18,
                      color: ObsidianColors.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    'Back to Library',
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.onSurface,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 16),

            // Breadcrumbs
            Text(
              'Library',
              style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
            ),
            Text(
              ' / ',
              style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
            ),
            Text(
              isMovie ? 'Movies' : 'TV Series',
              style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
            ),
            Text(
              ' / ',
              style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
            ),
            Text(
              show.title,
              style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
            ),
            const SizedBox(width: 8),

            // MAL ID Badge
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(4),
              ),
              child: Text(
                'MAL:${show.malId}',
                style: ObsidianTypography.labelBadge(
                  color: ObsidianColors.tertiary,
                ),
              ),
            ),

            const SizedBox(width: 24),

            // Download Season Button
            Tooltip(
              message: 'Download Season',
              child: FilledButton.icon(
                onPressed: () => state.downloadSeason(show.malId),
                icon: const Icon(Icons.download_rounded, size: 16),
                label: Text(isMovie ? 'Download Movie' : 'Download Season'),
                style: FilledButton.styleFrom(
                  backgroundColor: ObsidianColors.primaryContainer,
                  foregroundColor: ObsidianColors.onPrimaryContainer,
                  padding: const EdgeInsets.symmetric(
                    horizontal: 14,
                    vertical: 8,
                  ),
                  textStyle: ObsidianTypography.monoCode(
                    color: ObsidianColors.onPrimaryContainer,
                  ),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                ),
              ),
            ),
            const SizedBox(width: 8),

            // Refresh Metadata Button
            OutlinedButton.icon(
              onPressed: show.metadataStatus == 'fetching'
                  ? null
                  : () => state.refreshShowMetadata(show.malId, force: true),
              icon: show.metadataStatus == 'fetching'
                  ? const SizedBox(
                      width: 14,
                      height: 14,
                      child: CircularProgressIndicator(
                        strokeWidth: 2,
                        color: ObsidianColors.tertiary,
                      ),
                    )
                  : const Icon(
                      Icons.sync_rounded,
                      size: 16,
                      color: ObsidianColors.tertiary,
                    ),
              label: Text(
                show.metadataStatus == 'fetching'
                    ? 'Refreshing...'
                    : 'Refresh Metadata',
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.onSurface,
                ),
              ),
              style: OutlinedButton.styleFrom(
                backgroundColor: ObsidianColors.surfaceContainer,
                side: const BorderSide(color: ObsidianColors.border),
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 8,
                ),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(8),
                ),
              ),
            ),
            const SizedBox(width: 8),

            // Recheck Hashes Button
            OutlinedButton.icon(
              onPressed: () async {
                await state.bridge.rescanLibrary();
                await state.loadShowDetail(show.malId);
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(
                      content: Text('Library rescanned and hashes rechecked.'),
                    ),
                  );
                }
              },
              icon: const Icon(
                Icons.verified_outlined,
                size: 16,
                color: ObsidianColors.primary,
              ),
              label: Text(
                'Recheck Hashes',
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.onSurface,
                ),
              ),
              style: OutlinedButton.styleFrom(
                backgroundColor: ObsidianColors.surfaceContainer,
                side: const BorderSide(color: ObsidianColors.border),
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 8,
                ),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(8),
                ),
              ),
            ),
            const SizedBox(width: 8),

            // Delete from disk button
            Tooltip(
              message: 'Delete from disk',
              child: InkWell(
                onTap: () {
                  confirmDeleteShowDialog(context, state, show);
                },
                borderRadius: BorderRadius.circular(8),
                child: Container(
                  width: 32,
                  height: 32,
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: const Icon(
                    Icons.delete_outline,
                    size: 18,
                    color: ObsidianColors.outline,
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
