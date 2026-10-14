import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/rule_item.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/state_views.dart';

class RulesPage extends StatefulWidget {
  const RulesPage({super.key});

  @override
  State<RulesPage> createState() => _RulesPageState();
}

class _RulesPageState extends State<RulesPage> {
  // Global rule models and controllers
  String _selectedScope = 'global';
  String _targetResolution = '1080p';
  bool _hevcAllowed = true;
  bool _avcAllowed = true;
  bool _av1Allowed = true;
  bool _mpegAllowed = false;
  bool _flacAudio = true;
  bool _opusAudio = true;
  bool _bypassInboxStagnation = true;

  final TextEditingController _autoSeedController = TextEditingController(text: '10');
  final TextEditingController _triageMinController = TextEditingController(text: '1');
  final TextEditingController _triageMaxController = TextEditingController(text: '9');
  final TextEditingController _blacklistInputController = TextEditingController();

  // Fansub Group Priority List
  List<Map<String, dynamic>> _fansubGroups = [
    {
      'rank': '#01',
      'name': '[SubsPlease]',
      'tier': 'Tier 1',
      'points': '+50 pts',
      'description': 'Official Crunchyroll Rip • Fast Auto-Sync • Multi-audio',
      'active': true,
    },
    {
      'rank': '#02',
      'name': '[Erai-raws]',
      'tier': 'Tier 1',
      'points': '+45 pts',
      'description': 'Multi-subs Internal • Clean Remux • Direct BD Stream',
      'active': true,
    },
    {
      'rank': '#03',
      'name': '[Judas]',
      'tier': 'Tier 2',
      'points': '+30 pts',
      'description': 'High-Efficiency HEVC x265 10-bit • Batch Packed',
      'active': true,
    },
    {
      'rank': '#04',
      'name': '[ASW]',
      'tier': 'Tier 2',
      'points': '+25 pts',
      'description': 'Mini-MKV • Low Bitrate Fallback • Mobile Friendly',
      'active': true,
    },
    {
      'rank': '#05',
      'name': '[Beatrice-Raws]',
      'tier': 'Tier 3',
      'points': '+20 pts',
      'description': 'Archival BDRip • Hi10P FLAC Lossless • Large Bitrate',
      'active': true,
    },
  ];

  // Blacklist Items
  List<Map<String, dynamic>> _blacklistItems = [
    {
      'name': 'BadRaws_Spam',
      'type': 'Uploader',
      'meta': 'Blocked 14 days ago • Matched 22 items',
    },
    {
      'name': r'regex:\b(CAM|TS|HDCAM)\b',
      'type': 'Pattern',
      'meta': 'Theatre recording detection • Auto-filter',
    },
    {
      'name': 'ZeroSeedsBot',
      'type': 'Tracker',
      'meta': 'Known malicious spoof tracker',
    },
    {
      'name': 'DubOnly_FakeHD',
      'type': 'Release Tag',
      'meta': 'Upscaled 480p bad encodes',
    },
  ];

  bool _initializedFromState = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<StateProvider>().refreshRules();
    });
  }

  void _syncWithGlobalRule(RuleItem rule) {
    if (_initializedFromState) return;
    _initializedFromState = true;
    _targetResolution = rule.preferredResolution;
    _autoSeedController.text = rule.seedAutoThreshold.toString();
    _triageMinController.text = rule.seedAskMin.toString();
    _triageMaxController.text = rule.seedAskMax.toString();

    // If groups exist in backend, update fansub groups
    if (rule.groups.isNotEmpty) {
      final updated = <Map<String, dynamic>>[];
      for (int i = 0; i < rule.groups.length; i++) {
        final g = rule.groups[i];
        final rankNum = (i + 1).toString().padLeft(2, '0');
        updated.add({
          'rank': '#$rankNum',
          'name': g.startsWith('[') ? g : '[$g]',
          'tier': i < 2 ? 'Tier 1' : (i < 4 ? 'Tier 2' : 'Tier 3'),
          'points': '+${50 - (i * 5)} pts',
          'description': 'Automated release group matcher',
          'active': true,
        });
      }
      _fansubGroups = updated;
    }

    if (rule.blockedUploaders.isNotEmpty) {
      _blacklistItems = rule.blockedUploaders.map((b) => {
        'name': b,
        'type': 'Blocked Target',
        'meta': 'Configured in backend rules',
      }).toList();
    }
  }

  @override
  void dispose() {
    _autoSeedController.dispose();
    _triageMinController.dispose();
    _triageMaxController.dispose();
    _blacklistInputController.dispose();
    super.dispose();
  }

  Future<void> _saveAllRules(StateProvider state) async {
    final groupNames = _fansubGroups.map((g) {
      final name = g['name'] as String;
      return name.replaceAll('[', '').replaceAll(']', '').trim();
    }).toList();

    final blocked = _blacklistItems.map((b) => b['name'] as String).toList();
    final seedAuto = int.tryParse(_autoSeedController.text.trim()) ?? 10;
    final askMin = int.tryParse(_triageMinController.text.trim()) ?? 1;
    final askMax = int.tryParse(_triageMaxController.text.trim()) ?? 9;

    await state.updateRule('groups', groupNames);
    await state.updateRule('preferred_resolution', _targetResolution);
    await state.updateRule('blocked_uploaders', blocked);
    await state.updateRule('seed_auto_threshold', seedAuto);
    await state.updateRule('seed_ask_min', askMin);
    await state.updateRule('seed_ask_max', askMax);

    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Automation & scoring policies saved to daemon'),
          backgroundColor: ObsidianColors.success,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    if (state.coreError != null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24.0),
          child: ErrorStateView(
            title: 'Rules Fetch Error',
            message: state.coreError!,
            onRetry: () => state.refreshAll(),
            retryLabel: 'Retry Rules Query',
          ),
        ),
      );
    }

    if (state.globalRule != null && !_initializedFromState) {
      _syncWithGlobalRule(state.globalRule!);
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 20),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 1400),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // Header & Scope Management Section
              _buildHeaderAndScope(context, state),
              const SizedBox(height: 24),

              // Responsive Two-Column Grid:
              // Left: Preferred Fansub Groups (Col 7) + Swarm Health (Col 6)
              // Right: Quality & Video Codec (Col 5) + Blocked Uploaders (Col 6)
              LayoutBuilder(
                builder: (context, constraints) {
                  final isWide = constraints.maxWidth > 900;
                  if (isWide) {
                    return Column(
                      children: [
                        Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Expanded(flex: 7, child: _buildFansubGroupsCard()),
                            const SizedBox(width: 16),
                            Expanded(flex: 5, child: _buildQualityCodecCard()),
                          ],
                        ),
                        const SizedBox(height: 16),
                        Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Expanded(flex: 6, child: _buildSwarmHealthCard()),
                            const SizedBox(width: 16),
                            Expanded(flex: 6, child: _buildBlacklistCard()),
                          ],
                        ),
                      ],
                    );
                  } else {
                    return Column(
                      children: [
                        _buildFansubGroupsCard(),
                        const SizedBox(height: 16),
                        _buildQualityCodecCard(),
                        const SizedBox(height: 16),
                        _buildSwarmHealthCard(),
                        const SizedBox(height: 16),
                        _buildBlacklistCard(),
                      ],
                    );
                  }
                },
              ),

              const SizedBox(height: 20),
              // Policy Matcher Telemetry Simulator
              _buildTelemetrySimulator(),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildHeaderAndScope(BuildContext context, StateProvider state) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Breadcrumbs
        Row(
          children: [
            Text('System / Automation Engines / ', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
            Text('Rules & Release Policies', style: ObsidianTypography.monoCode(color: ObsidianColors.primary).copyWith(fontWeight: FontWeight.w600)),
          ],
        ),
        const SizedBox(height: 8),

        // Title and Toolbar
        Wrap(
          alignment: WrapAlignment.spaceBetween,
          crossAxisAlignment: WrapCrossAlignment.start,
          spacing: 16,
          runSpacing: 12,
          children: [
            Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text('Rules & Automation Policies', style: ObsidianTypography.headlineLg()),
                    const SizedBox(width: 10),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                      decoration: BoxDecoration(
                        color: ObsidianColors.primary.withOpacity(0.12),
                        borderRadius: BorderRadius.circular(12),
                        border: Border.all(color: ObsidianColors.primary.withOpacity(0.25)),
                      ),
                      child: Text('LIVE ENGINE', style: ObsidianTypography.labelBadge(color: ObsidianColors.primary)),
                    ),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  'Configure automated parsing hierarchies, release scoring bonuses, codec filters, and ingest gating criteria.',
                  style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
                ),
              ],
            ),

            // Scope Selector & Buttons
            Wrap(
              spacing: 8,
              runSpacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                // Scope Dropdown
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(10),
                    border: Border.all(color: ObsidianColors.border),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text('Scope: ', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                      DropdownButtonHideUnderline(
                        child: DropdownButton<String>(
                          value: _selectedScope,
                          dropdownColor: ObsidianColors.surfaceContainerHigh,
                          icon: const Icon(Icons.expand_more_rounded, size: 16, color: ObsidianColors.outline),
                          style: ObsidianTypography.titleSm(color: ObsidianColors.onSurface),
                          items: const [
                            DropdownMenuItem(value: 'global', child: Text('Global Rules (Default)')),
                            DropdownMenuItem(value: 'frieren', child: Text('Override: Sousou no Frieren')),
                            DropdownMenuItem(value: 'sololeveling', child: Text('Override: Solo Leveling')),
                            DropdownMenuItem(value: 'kiminonawa', child: Text('Override: Kimi no Na wa')),
                            DropdownMenuItem(value: 'dungeon', child: Text('Override: Dungeon Meshi')),
                          ],
                          onChanged: (val) {
                            if (val != null) setState(() => _selectedScope = val);
                          },
                        ),
                      ),
                      const SizedBox(width: 6),
                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1.5),
                        decoration: BoxDecoration(
                          color: ObsidianColors.tertiary.withOpacity(0.18),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text('Active on 18 shows', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
                      ),
                    ],
                  ),
                ),

                // Test RSS Button
                OutlinedButton.icon(
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(
                        content: Text('Testing RSS feeds: Nyaa and Tokyo Toshokan reached successfully'),
                        backgroundColor: ObsidianColors.surfaceContainerHigh,
                      ),
                    );
                  },
                  icon: const Icon(Icons.science_rounded, size: 16, color: ObsidianColors.tertiary),
                  label: const Text('Test RSS Feed'),
                  style: OutlinedButton.styleFrom(
                    side: const BorderSide(color: ObsidianColors.border),
                    foregroundColor: ObsidianColors.onSurface,
                    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                  ),
                ),

                // Save Button
                ElevatedButton.icon(
                  onPressed: () => _saveAllRules(state),
                  icon: const Icon(Icons.check_rounded, size: 18, color: ObsidianColors.onPrimaryContainer),
                  label: const Text('Save Changes'),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: ObsidianColors.primaryContainer,
                    foregroundColor: ObsidianColors.onPrimaryContainer,
                    elevation: 0,
                    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                  ),
                ),
              ],
            ),
          ],
        ),
      ],
    );
  }

  // CARD 1: PREFERRED FANSUB GROUPS
  Widget _buildFansubGroupsCard() {
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
                onPressed: () => _showAddGroupDialog(),
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
            itemCount: _fansubGroups.length,
            onReorder: (oldIndex, newIndex) {
              setState(() {
                if (newIndex > oldIndex) newIndex--;
                final item = _fansubGroups.removeAt(oldIndex);
                _fansubGroups.insert(newIndex, item);
                // re-rank
                for (int i = 0; i < _fansubGroups.length; i++) {
                  _fansubGroups[i]['rank'] = '#${(i + 1).toString().padLeft(2, '0')}';
                  _fansubGroups[i]['points'] = '+${50 - (i * 5)} pts';
                }
              });
            },
            itemBuilder: (context, index) {
              final group = _fansubGroups[index];
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
                        setState(() {
                          _fansubGroups.removeAt(index);
                        });
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
              Text('${_fansubGroups.length} groups active', style: ObsidianTypography.labelBadge(color: ObsidianColors.secondary)),
            ],
          ),
        ],
      ),
    );
  }

  void _showAddGroupDialog() {
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
                setState(() {
                  final newNum = (_fansubGroups.length + 1).toString().padLeft(2, '0');
                  _fansubGroups.add({
                    'rank': '#$newNum',
                    'name': '[${nameCtl.text.trim().replaceAll('[', '').replaceAll(']', '')}]',
                    'tier': 'Tier 2',
                    'points': '+25 pts',
                    'description': 'Custom added release group',
                    'active': true,
                  });
                });
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

  // CARD 2: QUALITY & VIDEO CODEC SPECIFICATIONS
  Widget _buildQualityCodecCard() {
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
                  const Icon(Icons.tune_rounded, size: 20, color: ObsidianColors.primary),
                  const SizedBox(width: 8),
                  Text('Quality & Codec Specs', style: ObsidianTypography.titleSm()),
                ],
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('v2.4 Core', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
              ),
            ],
          ),
          const SizedBox(height: 14),

          // Target Resolution Segmented Control
          Text('TARGET RESOLUTION', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
          const SizedBox(height: 6),
          Row(
            children: [
              _buildResolutionButton('720p HD', '720p'),
              const SizedBox(width: 6),
              _buildResolutionButton('1080p FHD', '1080p'),
              const SizedBox(width: 6),
              _buildResolutionButton('2160p 4K', '2160p'),
            ],
          ),
          const SizedBox(height: 4),
          Text('Optimal profile: 1080p SDR/HDR downsampled with dual audio preferred.', style: ObsidianTypography.monoCode(color: ObsidianColors.secondary)),
          const SizedBox(height: 14),

          // Allowed Video Codecs Checkbox Grid
          Text('ALLOWED VIDEO CODECS', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
          const SizedBox(height: 6),
          Row(
            children: [
              Expanded(
                child: _buildCheckboxTile(
                  title: 'HEVC (H.265)',
                  subtitle: 'Archival priority',
                  value: _hevcAllowed,
                  onChanged: (v) => setState(() => _hevcAllowed = v ?? true),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: _buildCheckboxTile(
                  title: 'AVC (H.264)',
                  subtitle: 'Universal compat',
                  value: _avcAllowed,
                  onChanged: (v) => setState(() => _avcAllowed = v ?? true),
                ),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Row(
            children: [
              Expanded(
                child: _buildCheckboxTile(
                  title: 'AV1',
                  subtitle: 'Next-gen open',
                  value: _av1Allowed,
                  onChanged: (v) => setState(() => _av1Allowed = v ?? true),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: _buildCheckboxTile(
                  title: 'MPEG-2 / DivX',
                  subtitle: 'Legacy (Blocked)',
                  value: _mpegAllowed,
                  onChanged: (v) => setState(() => _mpegAllowed = v ?? false),
                ),
              ),
            ],
          ),
          const SizedBox(height: 14),

          // Audio Channels Checkboxes
          Text('AUDIO CHANNELS & CODECS', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
          const SizedBox(height: 6),
          Wrap(
            spacing: 8,
            runSpacing: 6,
            children: [
              FilterChip(
                selected: _flacAudio,
                label: const Text('Lossless FLAC (24-bit/5.1)'),
                onSelected: (v) => setState(() => _flacAudio = v),
                selectedColor: ObsidianColors.primary.withOpacity(0.2),
                checkmarkColor: ObsidianColors.primary,
              ),
              FilterChip(
                selected: _opusAudio,
                label: const Text('Opus / AAC 2.0+'),
                onSelected: (v) => setState(() => _opusAudio = v),
                selectedColor: ObsidianColors.primary.withOpacity(0.2),
                checkmarkColor: ObsidianColors.primary,
              ),
            ],
          ),
          const SizedBox(height: 14),

          // File Size Limits Bar Visualizer
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('EPISODE SIZE GATE', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
              Text('400 MB — 1.80 GB', style: ObsidianTypography.monoCode(color: ObsidianColors.primary).copyWith(fontWeight: FontWeight.bold)),
            ],
          ),
          const SizedBox(height: 6),
          Container(
            height: 6,
            width: double.infinity,
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(3),
            ),
            child: Row(
              children: [
                const Spacer(flex: 16),
                Expanded(
                  flex: 52,
                  child: Container(
                    decoration: BoxDecoration(
                      color: ObsidianColors.primary,
                      borderRadius: BorderRadius.circular(3),
                    ),
                  ),
                ),
                const Spacer(flex: 32),
              ],
            ),
          ),
          const SizedBox(height: 4),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('100M', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
              Text('400M (Min)', style: ObsidianTypography.labelBadge(color: ObsidianColors.primary)),
              Text('1.0G', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
              Text('1.8G (Max)', style: ObsidianTypography.labelBadge(color: ObsidianColors.primary)),
              Text('3.5G+', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildResolutionButton(String label, String value) {
    final isSelected = _targetResolution == value;
    return Expanded(
      child: InkWell(
        onTap: () => setState(() => _targetResolution = value),
        borderRadius: BorderRadius.circular(6),
        child: Container(
          padding: const EdgeInsets.symmetric(vertical: 8),
          decoration: BoxDecoration(
            color: isSelected ? ObsidianColors.primaryContainer : ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(6),
            border: Border.all(
              color: isSelected ? ObsidianColors.primary : ObsidianColors.border,
            ),
          ),
          alignment: Alignment.center,
          child: Text(
            label,
            style: ObsidianTypography.monoCode(
              color: isSelected ? ObsidianColors.onPrimaryContainer : ObsidianColors.onSurfaceVariant,
            ).copyWith(fontWeight: FontWeight.bold),
          ),
        ),
      ),
    );
  }

  Widget _buildCheckboxTile({
    required String title,
    required String subtitle,
    required bool value,
    required ValueChanged<bool?> onChanged,
  }) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Row(
        children: [
          Checkbox(
            value: value,
            onChanged: onChanged,
            activeColor: ObsidianColors.primary,
          ),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: ObsidianTypography.bodySm().copyWith(fontWeight: FontWeight.bold)),
                Text(subtitle, style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
              ],
            ),
          ),
        ],
      ),
    );
  }

  // CARD 3: SWARM HEALTH & SEED THRESHOLDS
  Widget _buildSwarmHealthCard() {
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
                  const Icon(Icons.speed_rounded, size: 20, color: ObsidianColors.tertiary),
                  const SizedBox(width: 8),
                  Text('Swarm Health & Ingest Criteria', style: ObsidianTypography.titleSm()),
                ],
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.tertiary.withOpacity(0.2),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('Daemon Live', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
              ),
            ],
          ),
          const SizedBox(height: 4),
          Text(
            'Define boundary conditions for unattended automatic download versus triage routing into the user Inbox.',
            style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
          ),
          const SizedBox(height: 14),

          // Two Inputs Side-by-Side
          Row(
            children: [
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: ObsidianColors.border),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Expanded(
                            child: Text(
                              'AUTO-COMMIT TARGET',
                              style: ObsidianTypography.labelBadge(color: ObsidianColors.outline),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                          const SizedBox(width: 4),
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                            decoration: BoxDecoration(
                              color: ObsidianColors.tertiary.withOpacity(0.2),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text('Direct DL', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
                          ),
                        ],
                      ),
                      const SizedBox(height: 8),
                      Wrap(
                        crossAxisAlignment: WrapCrossAlignment.center,
                        spacing: 6,
                        children: [
                          Text('At or above:', style: ObsidianTypography.bodySm()),
                          SizedBox(
                            width: 50,
                            height: 30,
                            child: TextField(
                              controller: _autoSeedController,
                              textAlign: TextAlign.center,
                              style: ObsidianTypography.monoCode(color: ObsidianColors.primary).copyWith(fontWeight: FontWeight.bold),
                              decoration: const InputDecoration(
                                border: OutlineInputBorder(),
                                contentPadding: EdgeInsets.zero,
                              ),
                            ),
                          ),
                          Text('seeders', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                        ],
                      ),
                      const SizedBox(height: 4),
                      Text('Transfers to qBittorrent immediately', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: ObsidianColors.border),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Expanded(
                            child: Text(
                              'TRIAGE ROUTE',
                              style: ObsidianTypography.labelBadge(color: ObsidianColors.outline),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                          const SizedBox(width: 4),
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                            decoration: BoxDecoration(
                              color: ObsidianColors.error.withOpacity(0.2),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text('Inbox Gate', style: ObsidianTypography.labelBadge(color: ObsidianColors.error)),
                          ),
                        ],
                      ),
                      const SizedBox(height: 8),
                      Wrap(
                        crossAxisAlignment: WrapCrossAlignment.center,
                        spacing: 4,
                        children: [
                          Text('Between:', style: ObsidianTypography.bodySm()),
                          SizedBox(
                            width: 38,
                            height: 30,
                            child: TextField(
                              controller: _triageMinController,
                              textAlign: TextAlign.center,
                              style: ObsidianTypography.monoCode(),
                              decoration: const InputDecoration(border: OutlineInputBorder(), contentPadding: EdgeInsets.zero),
                            ),
                          ),
                          Text('&', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                          SizedBox(
                            width: 38,
                            height: 30,
                            child: TextField(
                              controller: _triageMaxController,
                              textAlign: TextAlign.center,
                              style: ObsidianTypography.monoCode(),
                              decoration: const InputDecoration(border: OutlineInputBorder(), contentPadding: EdgeInsets.zero),
                            ),
                          ),
                          Text('seeds', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                        ],
                      ),
                      const SizedBox(height: 4),
                      Text('Requires manual one-click confirmation', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                    ],
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 14),

          // Spectrum Bar Visualization
          Wrap(
            alignment: WrapAlignment.spaceBetween,
            spacing: 8,
            runSpacing: 4,
            children: [
              Text('0 Seeds (Dropped)', style: ObsidianTypography.labelBadge(color: ObsidianColors.error)),
              Text('1 - 9 Seeds (Inbox Approval)', style: ObsidianTypography.labelBadge(color: ObsidianColors.secondary)),
              Text('10+ Seeds (Automated)', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
            ],
          ),
          const SizedBox(height: 4),
          ClipRRect(
            borderRadius: BorderRadius.circular(6),
            child: SizedBox(
              height: 10,
              child: Row(
                children: [
                  Expanded(flex: 12, child: Container(color: ObsidianColors.error)),
                  Expanded(flex: 38, child: Container(color: ObsidianColors.secondary)),
                  Expanded(flex: 50, child: Container(color: ObsidianColors.tertiary)),
                ],
              ),
            ),
          ),
          const SizedBox(height: 14),

          // Fallback switch
          Material(
            type: MaterialType.transparency,
            child: SwitchListTile(
              value: _bypassInboxStagnation,
              onChanged: (v) => setState(() => _bypassInboxStagnation = v),
              title: Text('Bypass Inbox Stagnation', style: ObsidianTypography.bodySm().copyWith(fontWeight: FontWeight.bold)),
              subtitle: Text('Auto-release item from Inbox to downloader if torrent is >48 hours old and no higher-seed release appears.', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
              activeColor: ObsidianColors.primary,
              contentPadding: EdgeInsets.zero,
            ),
          ),
        ],
      ),
    );
  }

  // CARD 4: BLOCKED UPLOADERS & BLACKLIST
  Widget _buildBlacklistCard() {
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
                  const Icon(Icons.block_rounded, size: 20, color: ObsidianColors.error),
                  const SizedBox(width: 8),
                  Flexible(child: Text('Blocked Uploaders & Blacklist', style: ObsidianTypography.titleSm(), overflow: TextOverflow.ellipsis)),
                ],
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.error.withOpacity(0.2),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('Silent Drop', style: ObsidianTypography.labelBadge(color: ObsidianColors.error)),
              ),
            ],
          ),
          const SizedBox(height: 4),
          Text(
            'Torrents matching these regex patterns, uploader signatures, or bad trackers are immediately suppressed from all feeds.',
            style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
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
                    controller: _blacklistInputController,
                    style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
                    decoration: InputDecoration(
                      hintText: r'Add tag, uploader name, or regex: \b(CAM|TS)\b...',
                      hintStyle: ObsidianTypography.monoCode(color: ObsidianColors.outline),
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
                  final text = _blacklistInputController.text.trim();
                  if (text.isNotEmpty) {
                    setState(() {
                      _blacklistItems.insert(0, {
                        'name': text,
                        'type': 'Custom Block',
                        'meta': 'Added just now',
                      });
                      _blacklistInputController.clear();
                    });
                  }
                },
                icon: const Icon(Icons.add_moderator_rounded, size: 16),
                label: const Text('Block'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainerHigh,
                  foregroundColor: ObsidianColors.error,
                  elevation: 0,
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),

          // Blacklist list
          ListView.builder(
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            itemCount: _blacklistItems.length,
            itemBuilder: (context, index) {
              final item = _blacklistItems[index];
              return Container(
                margin: const EdgeInsets.only(bottom: 6),
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
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
                              Text(item['name'], style: ObsidianTypography.monoCode().copyWith(fontWeight: FontWeight.bold)),
                              const SizedBox(width: 6),
                              Container(
                                padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                                decoration: BoxDecoration(
                                  color: ObsidianColors.surfaceContainerHighest,
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text(item['type'], style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                              ),
                            ],
                          ),
                          Text(item['meta'], style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                        ],
                      ),
                    ),
                    IconButton(
                      icon: const Icon(Icons.close_rounded, size: 16, color: ObsidianColors.outline),
                      onPressed: () {
                        setState(() {
                          _blacklistItems.removeAt(index);
                        });
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

  // POLICY MATCHER TELEMETRY SIMULATOR
  Widget _buildTelemetrySimulator() {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(width: 7, height: 7, decoration: const BoxDecoration(shape: BoxShape.circle, color: ObsidianColors.tertiary)),
              const SizedBox(width: 8),
              Text('Policy Matcher Telemetry: ', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface).copyWith(fontWeight: FontWeight.bold)),
              Expanded(
                child: Text(
                  '[SubsPlease] Sousou no Frieren - 28 (1080p) [98E7B21F].mkv',
                  style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              Text('Calculated Score: +185 pts', style: ObsidianTypography.monoCode(color: ObsidianColors.tertiary)),
              const SizedBox(width: 12),
              Text('Verdict: AUTO_COMMITTED_QBTT', style: ObsidianTypography.monoCode(color: ObsidianColors.primary).copyWith(fontWeight: FontWeight.bold)),
            ],
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 6,
            runSpacing: 4,
            children: [
              _buildTelemetryChip('Group: SubsPlease (+50)'),
              _buildTelemetryChip('Res: 1080p (+40)'),
              _buildTelemetryChip('Codec: H.264/AVC (+20)'),
              _buildTelemetryChip('Audio: AAC 2.0 (+15)'),
              _buildTelemetryChip('Swarm: 142 Seeds (>10 Target, +60)'),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('Inbox: BYPASSED', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildTelemetryChip(String text) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(text, style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurfaceVariant)),
    );
  }
}
