import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class RulesFansubGroupsCard extends StatelessWidget {
  final List<Map<String, dynamic>> fansubGroups;
  final ValueChanged<List<Map<String, dynamic>>> onGroupsChanged;

  const RulesFansubGroupsCard({
    super.key,
    required this.fansubGroups,
    required this.onGroupsChanged,
  });

  void _showAddGroupDialog(BuildContext context) {
    final nameCtl = TextEditingController();
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: ObsidianColors.surfaceContainerLow,
        title: Text('Add Fansub Group', style: ObsidianTypography.titleSm()),
        content: TextField(
          controller: nameCtl,
          style: ObsidianTypography.bodyMd(),
          decoration: const InputDecoration(
            hintText: 'e.g. SubsPlease or Erai-raws',
            border: OutlineInputBorder(),
          ),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
          ElevatedButton(
            onPressed: () {
              if (nameCtl.text.trim().isNotEmpty) {
                final updated = List<Map<String, dynamic>>.from(fansubGroups);
                final newNum = (updated.length + 1).toString().padLeft(2, '0');
                updated.add({
                  'rank': '#$newNum',
                  'name': '[${nameCtl.text.trim().replaceAll('[', '').replaceAll(']', '')}]',
                  'tier': 'Tier 2',
                  'points': '+25 pts',
                  'description': 'Custom added release group',
                  'active': true,
                });
                onGroupsChanged(updated);
                Navigator.pop(ctx);
              }
            },
            style: ElevatedButton.styleFrom(backgroundColor: ObsidianColors.primaryContainer),
            child: const Text('Add'),
          ),
        ],
      ),
    );
  }

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
                  const Icon(Icons.groups_rounded, size: 20, color: ObsidianColors.primary),
                  const SizedBox(width: 8),
                  Text('Preferred Fansub Groups & Scoring Priority', style: ObsidianTypography.titleSm()),
                ],
              ),
              OutlinedButton.icon(
                onPressed: () => _showAddGroupDialog(context),
                icon: const Icon(Icons.add_rounded, size: 14, color: ObsidianColors.primary),
                label: const Text('Add Group'),
                style: OutlinedButton.styleFrom(
                  side: const BorderSide(color: ObsidianColors.border),
                  foregroundColor: ObsidianColors.primary,
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                  minimumSize: Size.zero,
                  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                ),
              ),
            ],
          ),
          const SizedBox(height: 4),
          Text(
            'Ranked order determines ingestion bonus score. Top-tier releases receive auto-commit priority and bypass wait timers.',
            style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
          ),
          const SizedBox(height: 14),

          // Reorderable Group List
          ReorderableListView.builder(
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            itemCount: fansubGroups.length,
            onReorder: (oldIndex, newIndex) {
              final updated = List<Map<String, dynamic>>.from(fansubGroups);
              if (newIndex > oldIndex) newIndex--;
              final item = updated.removeAt(oldIndex);
              updated.insert(newIndex, item);
              // re-rank
              for (int i = 0; i < updated.length; i++) {
                updated[i]['rank'] = '#${(i + 1).toString().padLeft(2, '0')}';
                updated[i]['points'] = '+${50 - (i * 5)} pts';
              }
              onGroupsChanged(updated);
            },
            itemBuilder: (context, index) {
              final group = fansubGroups[index];
              return Container(
                key: ValueKey(group['name']),
                margin: const EdgeInsets.only(bottom: 8),
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Row(
                  children: [
                    const Icon(Icons.drag_indicator_rounded, size: 18, color: ObsidianColors.outline),
                    const SizedBox(width: 8),
                    Text(group['rank'], style: ObsidianTypography.monoCode(color: ObsidianColors.tertiary).copyWith(fontWeight: FontWeight.bold)),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Text(group['name'], style: ObsidianTypography.titleSm().copyWith(fontWeight: FontWeight.bold)),
                              const SizedBox(width: 6),
                              Container(
                                padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
                                decoration: BoxDecoration(
                                  color: ObsidianColors.secondary.withOpacity(0.18),
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text(group['tier'], style: ObsidianTypography.labelBadge(color: ObsidianColors.secondary)),
                              ),
                              const SizedBox(width: 6),
                              Text(group['points'], style: ObsidianTypography.monoCode(color: ObsidianColors.primary).copyWith(fontWeight: FontWeight.bold)),
                            ],
                          ),
                          const SizedBox(height: 2),
                          Text(group['description'], style: ObsidianTypography.bodySm(color: ObsidianColors.outline), maxLines: 1, overflow: TextOverflow.ellipsis),
                        ],
                      ),
                    ),
                    IconButton(
                      icon: const Icon(Icons.delete_outline_rounded, size: 18, color: ObsidianColors.outline),
                      onPressed: () {
                        final updated = List<Map<String, dynamic>>.from(fansubGroups);
                        updated.removeAt(index);
                        onGroupsChanged(updated);
                      },
                      hoverColor: ObsidianColors.error.withOpacity(0.15),
                    ),
                  ],
                ),
              );
            },
          ),
          const SizedBox(height: 8),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text(
                  'Drag handles to re-rank priority. Top match wins RSS race conditions.',
                  style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: 8),
              Text('${fansubGroups.length} groups active', style: ObsidianTypography.labelBadge(color: ObsidianColors.secondary)),
            ],
          ),
        ],
      ),
    );
  }
}
