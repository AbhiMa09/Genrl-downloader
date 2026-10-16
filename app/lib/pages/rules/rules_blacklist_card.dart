import 'package:flutter/material.dart';

import '../../theme/colors.dart';
import '../../theme/typography.dart';

class RulesBlacklistCard extends StatelessWidget {
  final TextEditingController blacklistInputController;
  final List<Map<String, dynamic>> blacklistItems;
  final ValueChanged<List<Map<String, dynamic>>> onBlacklistChanged;

  const RulesBlacklistCard({
    super.key,
    required this.blacklistInputController,
    required this.blacklistItems,
    required this.onBlacklistChanged,
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
                  const Icon(
                    Icons.block_rounded,
                    size: 20,
                    color: ObsidianColors.error,
                  ),
                  const SizedBox(width: 8),
                  Flexible(
                    child: Text(
                      'Blocked Uploaders & Blacklist',
                      style: ObsidianTypography.titleSm(),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                ],
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.error.withOpacity(0.2),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  'Silent Drop',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.error,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 4),
          Text(
            'Torrents matching these regex patterns, uploader signatures, or bad trackers are immediately suppressed from all feeds.',
            style: ObsidianTypography.bodySm(
              color: ObsidianColors.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 12),

          // Add input bar
          Row(
            children: [
              Expanded(
                child: Container(
                  height: 36,
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: ObsidianColors.border),
                  ),
                  child: TextField(
                    controller: blacklistInputController,
                    style: ObsidianTypography.monoCode(
                      color: ObsidianColors.onSurface,
                    ),
                    decoration: InputDecoration(
                      hintText:
                          r'Add tag, uploader name, or regex: \b(CAM|TS)\b...',
                      hintStyle: ObsidianTypography.monoCode(
                        color: ObsidianColors.outline,
                      ),
                      border: InputBorder.none,
                      isDense: true,
                      contentPadding: const EdgeInsets.symmetric(vertical: 8),
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 8),
              ElevatedButton.icon(
                onPressed: () {
                  final text = blacklistInputController.text.trim();
                  if (text.isNotEmpty) {
                    final updated = List<Map<String, dynamic>>.from(
                      blacklistItems,
                    );
                    updated.insert(0, {
                      'name': text,
                      'type': 'Custom Block',
                      'meta': 'Added just now',
                    });
                    blacklistInputController.clear();
                    onBlacklistChanged(updated);
                  }
                },
                icon: const Icon(Icons.add_moderator_rounded, size: 16),
                label: const Text('Block'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainerHigh,
                  foregroundColor: ObsidianColors.error,
                  elevation: 0,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),

          // Blacklist list
          ListView.builder(
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            itemCount: blacklistItems.length,
            itemBuilder: (context, index) {
              final item = blacklistItems[index];
              return Container(
                margin: const EdgeInsets.only(bottom: 6),
                padding: const EdgeInsets.symmetric(
                  horizontal: 10,
                  vertical: 8,
                ),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Row(
                  children: [
                    Container(
                      width: 6,
                      height: 6,
                      decoration: const BoxDecoration(
                        shape: BoxShape.circle,
                        color: ObsidianColors.error,
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Text(
                                item['name']?.toString() ?? '',
                                style: ObsidianTypography.monoCode().copyWith(
                                  fontWeight: FontWeight.bold,
                                ),
                              ),
                              const SizedBox(width: 6),
                              Container(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 4,
                                  vertical: 1,
                                ),
                                decoration: BoxDecoration(
                                  color: ObsidianColors.surfaceContainerHighest,
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text(
                                  item['type']?.toString() ?? '',
                                  style: ObsidianTypography.labelBadge(
                                    color: ObsidianColors.outline,
                                  ),
                                ),
                              ),
                            ],
                          ),
                          Text(
                            item['meta']?.toString() ?? '',
                            style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.outline,
                            ),
                          ),
                        ],
                      ),
                    ),
                    IconButton(
                      icon: const Icon(
                        Icons.close_rounded,
                        size: 16,
                        color: ObsidianColors.outline,
                      ),
                      onPressed: () {
                        final updated = List<Map<String, dynamic>>.from(
                          blacklistItems,
                        );
                        updated.removeAt(index);
                        onBlacklistChanged(updated);
                      },
                    ),
                  ],
                ),
              );
            },
          ),
        ],
      ),
    );
  }
}
