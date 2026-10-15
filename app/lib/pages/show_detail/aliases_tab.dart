import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class ShowDetailAliasesTab extends StatefulWidget {
  final LibraryShow show;
  final List<String> aliases;

  const ShowDetailAliasesTab({
    super.key,
    required this.show,
    required this.aliases,
  });

  @override
  State<ShowDetailAliasesTab> createState() => _ShowDetailAliasesTabState();
}

class _ShowDetailAliasesTabState extends State<ShowDetailAliasesTab> {
  late final TextEditingController _aliasController;
  late final TextEditingController _offsetController;

  @override
  void initState() {
    super.initState();
    _aliasController = TextEditingController();
    _offsetController = TextEditingController(text: widget.show.episodeOffset.toString());
  }

  @override
  void didUpdateWidget(covariant ShowDetailAliasesTab oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.show.malId != widget.show.malId || oldWidget.show.episodeOffset != widget.show.episodeOffset) {
      _offsetController.text = widget.show.episodeOffset.toString();
    }
  }

  @override
  void dispose() {
    _aliasController.dispose();
    _offsetController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();
    final show = widget.show;
    final aliases = widget.aliases;

    return Container(
      padding: const EdgeInsets.all(24),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(16),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('Title Aliases & Release Matching', style: ObsidianTypography.headlineMd()),
          const SizedBox(height: 4),
          Text(
            'Define alternate tracker queries and release names. When torrent releases match any alias below, they will automatically be parsed into this show directory.',
            style: ObsidianTypography.bodyMd(color: ObsidianColors.onSurfaceVariant),
          ),
          const SizedBox(height: 20),

          Text('CONFIGURED MATCH ALIASES', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
          const SizedBox(height: 10),

          // Chips of aliases
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: aliases.map((alias) {
              return Container(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(alias, style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface)),
                    const SizedBox(width: 8),
                    InkWell(
                      onTap: () async {
                        await state.removeAlias(show.malId, alias);
                      },
                      child: const Icon(Icons.close_rounded, size: 14, color: ObsidianColors.outline),
                    ),
                  ],
                ),
              );
            }).toList(),
          ),
          const SizedBox(height: 16),

          // Input for new alias
          Row(
            children: [
              Expanded(
                child: Container(
                  constraints: const BoxConstraints(maxWidth: 420),
                  child: TextField(
                    controller: _aliasController,
                    style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
                    decoration: InputDecoration(
                      hintText: 'Type new alias (e.g. ${show.title} S2)...',
                      hintStyle: ObsidianTypography.monoCode(color: ObsidianColors.outline),
                      filled: true,
                      fillColor: ObsidianColors.surfaceContainerLowest,
                      contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(8),
                        borderSide: BorderSide.none,
                      ),
                    ),
                    onSubmitted: (val) async {
                      if (val.trim().isNotEmpty) {
                        await state.addAlias(show.malId, val.trim());
                        _aliasController.clear();
                      }
                    },
                  ),
                ),
              ),
              const SizedBox(width: 10),
              FilledButton.icon(
                onPressed: () async {
                  final text = _aliasController.text.trim();
                  if (text.isNotEmpty) {
                    await state.addAlias(show.malId, text);
                    _aliasController.clear();
                  }
                },
                icon: const Icon(Icons.add, size: 16),
                label: const Text('Add Alias'),
                style: FilledButton.styleFrom(
                  backgroundColor: ObsidianColors.primary,
                  foregroundColor: ObsidianColors.onPrimary,
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
              ),
            ],
          ),
          const SizedBox(height: 24),

          // Episode Offset Box
          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLow,
              borderRadius: BorderRadius.circular(12),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('Episode Number Offset', style: ObsidianTypography.titleSm()),
                      Text(
                        'Adjust parsing integer if tracker indexes this anime as an ongoing sequel episode.',
                        style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 16),
                Row(
                  children: [
                    Text('Offset Value:', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                    const SizedBox(width: 8),
                    SizedBox(
                      width: 72,
                      child: TextField(
                        controller: _offsetController,
                        keyboardType: TextInputType.number,
                        textAlign: TextAlign.center,
                        style: ObsidianTypography.monoMetric(color: ObsidianColors.primary).copyWith(fontWeight: FontWeight.bold),
                        decoration: InputDecoration(
                          filled: true,
                          fillColor: ObsidianColors.surfaceContainerLowest,
                          contentPadding: const EdgeInsets.symmetric(vertical: 8),
                          border: OutlineInputBorder(borderRadius: BorderRadius.circular(8), borderSide: BorderSide.none),
                        ),
                        onSubmitted: (val) async {
                          final parsed = int.tryParse(val.trim());
                          if (parsed != null) {
                            await state.setEpisodeOffset(show.malId, parsed);
                          }
                        },
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
