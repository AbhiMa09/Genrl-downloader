import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../models/library_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class ShowDetailRulesTab extends StatefulWidget {
  final LibraryShow show;

  const ShowDetailRulesTab({
    super.key,
    required this.show,
  });

  @override
  State<ShowDetailRulesTab> createState() => _ShowDetailRulesTabState();
}

class _ShowDetailRulesTabState extends State<ShowDetailRulesTab> {
  String _preferredResolution = '1080p';
  String _audioProfile = 'Dual Audio FLAC';
  bool _autoHardlink = true;
  bool _isSavingRules = false;

  @override
  void initState() {
    super.initState();
    _initRulesFromShow(widget.show);
  }

  void _initRulesFromShow(LibraryShow show) {
    if (show.rules != null) {
      final res = show.rules!['preferred_resolution']?.toString();
      if (res != null && res.isNotEmpty) {
        _preferredResolution = res;
      }
      final moveMode = show.rules!['move_mode']?.toString();
      if (moveMode != null) {
        _autoHardlink = moveMode.toLowerCase() == 'hardlink';
      }
    }
  }

  @override
  void didUpdateWidget(covariant ShowDetailRulesTab oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.show.malId != widget.show.malId) {
      _initRulesFromShow(widget.show);
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();
    final show = widget.show;

    return Container(
      padding: const EdgeInsets.all(24),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(16),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('Per-Show Quality & Ingestion Overrides', style: ObsidianTypography.headlineMd()),
          const SizedBox(height: 4),
          Text.rich(
            TextSpan(
              text: 'Granular automated rules specific to ',
              style: ObsidianTypography.bodyMd(color: ObsidianColors.onSurfaceVariant),
              children: [
                TextSpan(
                  text: show.title,
                  style: ObsidianTypography.bodyMd(color: ObsidianColors.onSurface).copyWith(fontWeight: FontWeight.bold),
                ),
                const TextSpan(text: ' that supersede global daemon library configs.'),
              ],
            ),
          ),
          const SizedBox(height: 20),

          // Overrides Grid
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Resolution Preference
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(14),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('RESOLUTION PREFERENCE', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                      const SizedBox(height: 8),
                      DropdownButtonFormField<String>(
                        value: _preferredResolution,
                        dropdownColor: ObsidianColors.surfaceContainerLowest,
                        decoration: InputDecoration(
                          filled: true,
                          fillColor: ObsidianColors.surfaceContainerLowest,
                          border: OutlineInputBorder(borderRadius: BorderRadius.circular(8), borderSide: BorderSide.none),
                          contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                        ),
                        style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
                        items: const [
                          DropdownMenuItem(value: '1080p', child: Text('BD 1080p Remux (Selected)')),
                          DropdownMenuItem(value: '2160p', child: Text('4K UHD Remux (HDR10 / Dolby Vision)')),
                          DropdownMenuItem(value: '720p', child: Text('720p BDRip (Re-encode)')),
                          DropdownMenuItem(value: '480p', child: Text('WEB-DL 480p')),
                        ],
                        onChanged: (val) {
                          if (val != null) setState(() => _preferredResolution = val);
                        },
                      ),
                      const SizedBox(height: 6),
                      Text('Accepts bitrates ≥ 16.0 Mbps', style: ObsidianTypography.monoCode(color: ObsidianColors.tertiary).copyWith(fontSize: 10)),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 16),

              // Audio Codec Profile
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(14),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('AUDIO CODEC PROFILE', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                      const SizedBox(height: 8),
                      DropdownButtonFormField<String>(
                        value: _audioProfile,
                        dropdownColor: ObsidianColors.surfaceContainerLowest,
                        decoration: InputDecoration(
                          filled: true,
                          fillColor: ObsidianColors.surfaceContainerLowest,
                          border: OutlineInputBorder(borderRadius: BorderRadius.circular(8), borderSide: BorderSide.none),
                          contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                        ),
                        style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
                        items: const [
                          DropdownMenuItem(value: 'Dual Audio FLAC', child: Text('Dual Audio FLAC (Lossless 5.1/2.0)')),
                          DropdownMenuItem(value: 'Original Japanese Only', child: Text('Original Japanese Only (FLAC/TrueHD)')),
                          DropdownMenuItem(value: 'English Dub Primary', child: Text('English Dub Primary (AAC/AC3)')),
                        ],
                        onChanged: (val) {
                          if (val != null) setState(() => _audioProfile = val);
                        },
                      ),
                      const SizedBox(height: 6),
                      Text('Prefers Japanese FLAC default track', style: ObsidianTypography.monoCode(color: ObsidianColors.outline).copyWith(fontSize: 10)),
                    ],
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),

          // Release Group Priority Hierarchy
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLow,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text('RELEASE GROUP PRIORITY HIERARCHY', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                    Text('Left-to-Right Preference Order', style: ObsidianTypography.monoCode(color: ObsidianColors.primary).copyWith(fontSize: 10)),
                  ],
                ),
                const SizedBox(height: 10),
                Container(
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLowest,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: SingleChildScrollView(
                    scrollDirection: Axis.horizontal,
                    child: Builder(builder: (context) {
                      final groupsList = (show.rules?['groups'] as List<dynamic>?)?.map((e) => e.toString()).toList() ?? [];
                      if (groupsList.isEmpty) {
                        return Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                          child: Text('Default global group order (Auto, SubsPlease, Erai-raws, Judas)',
                              style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10)),
                        );
                      }
                      return Row(
                        children: [
                          for (int i = 0; i < groupsList.length; i++) ...[
                            if (i > 0) _buildGroupSeparator(),
                            _buildGroupPill('${i + 1}. ${groupsList[i]}', isPrimary: i == 0),
                          ],
                        ],
                      );
                    }),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),

          // Auto-Ingestion & Hardlink Switch
          Container(
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLow,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Daemon Auto-Ingestion & Hardlink', style: ObsidianTypography.titleSm()),
                    Text(
                      'Automatically hardlink from /mnt/torrents/completed directly into library when complete.',
                      style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
                    ),
                  ],
                ),
                Switch(
                  value: _autoHardlink,
                  activeColor: ObsidianColors.primary,
                  onChanged: (val) => setState(() => _autoHardlink = val),
                ),
              ],
            ),
          ),
          const SizedBox(height: 20),

          // Save settings action bar
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              OutlinedButton(
                onPressed: () async {
                  await state.bridge.resetRule(malId: show.malId);
                  await state.loadShowDetail(show.malId);
                  setState(() {
                    _preferredResolution = '1080p';
                    _autoHardlink = true;
                  });
                  if (context.mounted) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('Reset to global defaults.')),
                    );
                  }
                },
                style: OutlinedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainerHigh,
                  foregroundColor: ObsidianColors.onSurface,
                  side: const BorderSide(color: ObsidianColors.border),
                  padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 12),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
                child: const Text('Reset to Global Defaults'),
              ),
              const SizedBox(width: 12),
              FilledButton(
                onPressed: _isSavingRules
                    ? null
                    : () async {
                        setState(() => _isSavingRules = true);
                        try {
                          await state.updateRule('preferred_resolution', _preferredResolution, malId: show.malId);
                          await state.updateRule('move_mode', _autoHardlink ? 'hardlink' : 'copy', malId: show.malId);
                          await state.loadShowDetail(show.malId);
                          if (context.mounted) {
                            ScaffoldMessenger.of(context).showSnackBar(
                              const SnackBar(content: Text('Saved rule overrides.')),
                            );
                          }
                        } finally {
                          if (mounted) setState(() => _isSavingRules = false);
                        }
                      },
                style: FilledButton.styleFrom(
                  backgroundColor: ObsidianColors.primary,
                  foregroundColor: ObsidianColors.onPrimary,
                  padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
                child: _isSavingRules
                    ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2))
                    : const Text('Save Rule Overrides'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildGroupPill(String name, {bool isPrimary = false}) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: isPrimary ? ObsidianColors.primaryContainer : ObsidianColors.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(
        name,
        style: ObsidianTypography.monoCode(
          color: isPrimary ? ObsidianColors.onPrimaryContainer : ObsidianColors.onSurface,
        ).copyWith(fontWeight: isPrimary ? FontWeight.bold : FontWeight.normal),
      ),
    );
  }

  Widget _buildGroupSeparator() {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 6),
      child: Text('›', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
    );
  }
}
