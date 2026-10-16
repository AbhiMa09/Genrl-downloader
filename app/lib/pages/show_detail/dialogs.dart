import 'package:flutter/material.dart';

import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

void confirmDeleteShowDialog(
  BuildContext context,
  StateProvider state,
  LibraryShow show,
) {
  showDialog<void>(
    context: context,
    builder: (ctx) => AlertDialog(
      backgroundColor: ObsidianColors.surfaceContainer,
      title: Text(
        'Delete "${show.title}"?',
        style: ObsidianTypography.titleSm(),
      ),
      content: Text(
        'This will remove the show tracking from your library. If files exist on disk, they will be preserved unless explicitly purged.',
        style: ObsidianTypography.bodySm(),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(ctx).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () {
            Navigator.of(ctx).pop();
            state.closeShowDetail();
          },
          style: FilledButton.styleFrom(
            backgroundColor: ObsidianColors.errorContainer,
          ),
          child: const Text(
            'Remove from Library',
            style: TextStyle(color: ObsidianColors.onErrorContainer),
          ),
        ),
      ],
    ),
  );
}

void showRatingDialog(
  BuildContext context,
  StateProvider state,
  LibraryShow show,
) {
  double selectedRating = show.score;
  showDialog<void>(
    context: context,
    builder: (ctx) => StatefulBuilder(
      builder: (context, setDialogState) => AlertDialog(
        backgroundColor: ObsidianColors.surfaceContainer,
        title: Row(
          children: [
            const Icon(
              Icons.star_rounded,
              color: ObsidianColors.warning,
              size: 24,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                'Rate "${show.title}"',
                style: ObsidianTypography.titleSm(),
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              '★ ${selectedRating.toStringAsFixed(1)} / 10.0',
              style: ObsidianTypography.headlineLg(
                color: ObsidianColors.warning,
              ),
            ),
            const SizedBox(height: 16),
            Slider(
              value: selectedRating.clamp(1.0, 10.0),
              min: 1.0,
              max: 10.0,
              divisions: 18,
              label: selectedRating.toStringAsFixed(1),
              activeColor: ObsidianColors.warning,
              onChanged: (val) {
                setDialogState(() => selectedRating = val);
              },
            ),
            const SizedBox(height: 8),
            Wrap(
              spacing: 6,
              runSpacing: 6,
              children: [1, 5, 7, 8, 9, 10].map((star) {
                final isCurrent = selectedRating.round() == star;
                return ActionChip(
                  label: Text(
                    '★ $star',
                    style: ObsidianTypography.labelBadge(
                      color: isCurrent
                          ? ObsidianColors.warning
                          : ObsidianColors.onSurface,
                    ),
                  ),
                  onPressed: () {
                    setDialogState(() => selectedRating = star.toDouble());
                  },
                  backgroundColor: isCurrent
                      ? ObsidianColors.warning.withOpacity(0.2)
                      : ObsidianColors.surfaceContainerHighest,
                );
              }).toList(),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () async {
              await state.updateShowDetails(
                malId: show.malId,
                rating: selectedRating,
              );
              if (ctx.mounted) Navigator.of(ctx).pop();
              if (context.mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: Text(
                      'Updated rating to ★${selectedRating.toStringAsFixed(1)}',
                    ),
                    backgroundColor: ObsidianColors.success,
                  ),
                );
              }
            },
            style: FilledButton.styleFrom(
              backgroundColor: ObsidianColors.primary,
            ),
            child: const Text('Save Rating'),
          ),
        ],
      ),
    ),
  );
}

void showChangeMatchDialog(
  BuildContext context,
  StateProvider state,
  LibraryShow show,
) {
  final folderCtrl = TextEditingController(text: show.folderPath ?? '');
  showDialog<void>(
    context: context,
    builder: (ctx) => AlertDialog(
      backgroundColor: ObsidianColors.surfaceContainer,
      title: Text(
        'Change Show Match / Path',
        style: ObsidianTypography.titleSm(),
      ),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Set Custom Folder Directory Path:',
            style: ObsidianTypography.bodySm(),
          ),
          const SizedBox(height: 8),
          TextField(
            controller: folderCtrl,
            style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
            decoration: InputDecoration(
              filled: true,
              fillColor: ObsidianColors.surfaceContainerLowest,
              hintText: '/mnt/anime/...',
              hintStyle: ObsidianTypography.monoCode(),
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(8),
              ),
            ),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(ctx).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () async {
            final newPath = folderCtrl.text.trim();
            Navigator.of(ctx).pop();
            if (newPath.isNotEmpty) {
              await state.saveShowFolder(show.malId, newPath);
            }
          },
          child: const Text('Save Path'),
        ),
      ],
    ),
  );
}
