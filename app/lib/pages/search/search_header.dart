import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class SearchHeader extends StatelessWidget {
  final StateProvider state;
  final ValueChanged<String> onSelectPreset;

  const SearchHeader({
    super.key,
    required this.state,
    required this.onSelectPreset,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Wrap(
          alignment: WrapAlignment.spaceBetween,
          crossAxisAlignment: WrapCrossAlignment.center,
          spacing: 16,
          runSpacing: 10,
          children: [
            Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      'Manual Indexer Search',
                      style: ObsidianTypography.headlineLg(),
                    ),
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 6,
                        vertical: 2,
                      ),
                      decoration: BoxDecoration(
                        color: ObsidianColors.primary.withOpacity(0.12),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        'V2.4 CORE',
                        style: ObsidianTypography.labelBadge(
                          color: ObsidianColors.primary,
                          fontSize: 10,
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  'Direct query via Prowlarr / Nyaa.si API with automatic rule scoring and hash inspection',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.outline,
                  ),
                ),
              ],
            ),

            // Indexer Status Chips
            Wrap(
              spacing: 8,
              runSpacing: 6,
              children: [
                _buildIndexerChip(
                  'Nyaa.si',
                  state.isCoreConnected ? 'Online (Gate 2 req/s)' : 'Offline',
                  state.isCoreConnected
                      ? ObsidianColors.tertiary
                      : ObsidianColors.error,
                ),
                _buildIndexerChip(
                  'MyAnimeList',
                  state.isMalConnected ? 'OAuth Linked' : 'Public Profile',
                  state.isMalConnected
                      ? ObsidianColors.primary
                      : ObsidianColors.outline,
                ),
              ],
            ),
          ],
        ),
        if (state.shows.isNotEmpty) ...[
          const SizedBox(height: 12),
          _buildPresets(state),
        ],
      ],
    );
  }

  Widget _buildPresets(StateProvider state) {
    final presets = state.shows.take(5).map((s) => s.title).toList();

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: Row(
        children: [
          Row(
            children: [
              const Icon(
                Icons.history_rounded,
                size: 14,
                color: ObsidianColors.outline,
              ),
              const SizedBox(width: 4),
              Text(
                'PRESETS: ',
                style: ObsidianTypography.labelBadge(
                  color: ObsidianColors.outline,
                  fontSize: 9,
                ),
              ),
            ],
          ),
          const SizedBox(width: 6),
          ...presets.map((p) {
            return Padding(
              padding: const EdgeInsets.only(right: 6.0),
              child: InkWell(
                onTap: () => onSelectPreset(p),
                borderRadius: BorderRadius.circular(6),
                child: Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 4,
                  ),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHigh,
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(
                      color: ObsidianColors.border,
                      width: 0.5,
                    ),
                  ),
                  child: Text(
                    p,
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.onSurface,
                      fontSize: 10,
                    ),
                  ),
                ),
              ),
            );
          }),
        ],
      ),
    );
  }

  Widget _buildIndexerChip(String name, String status, Color dotColor) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: ObsidianColors.border, width: 0.5),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 6,
            height: 6,
            decoration: BoxDecoration(color: dotColor, shape: BoxShape.circle),
          ),
          const SizedBox(width: 6),
          Text(
            name,
            style: ObsidianTypography.monoCode(
              color: ObsidianColors.onSurface,
              fontSize: 10,
            ),
          ),
          const SizedBox(width: 4),
          Text(
            status,
            style: ObsidianTypography.monoCode(color: dotColor, fontSize: 9),
          ),
        ],
      ),
    );
  }
}
