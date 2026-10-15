import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../models/rule_item.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../widgets/state_views.dart';
import 'rules_blacklist_card.dart';
import 'rules_fansub_groups_card.dart';
import 'rules_header_scope.dart';
import 'rules_quality_codec_card.dart';
import 'rules_swarm_health_card.dart';
import 'rules_telemetry_simulator.dart';

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
      'name': '[Generic]',
      'tier': 'Tier 3',
      'points': '+20 pts',
      'description': 'Standard MKV / MP4 release group',
      'active': true,
    },
  ];

  // Blacklist Items
  List<Map<String, dynamic>> _blacklistItems = [];

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
              RulesHeaderScope(
                state: state,
                selectedScope: _selectedScope,
                onScopeChanged: (val) => setState(() => _selectedScope = val),
                onSave: () => _saveAllRules(state),
              ),
              const SizedBox(height: 24),

              // Responsive Two-Column Grid:
              LayoutBuilder(
                builder: (context, constraints) {
                  final isWide = constraints.maxWidth > 900;
                  if (isWide) {
                    return Column(
                      children: [
                        Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Expanded(
                              flex: 7,
                              child: RulesFansubGroupsCard(
                                fansubGroups: _fansubGroups,
                                onGroupsChanged: (val) => setState(() => _fansubGroups = val),
                              ),
                            ),
                            const SizedBox(width: 16),
                            Expanded(
                              flex: 5,
                              child: RulesQualityCodecCard(
                                targetResolution: _targetResolution,
                                onResolutionChanged: (val) => setState(() => _targetResolution = val),
                                hevcAllowed: _hevcAllowed,
                                onHevcChanged: (val) => setState(() => _hevcAllowed = val),
                                avcAllowed: _avcAllowed,
                                onAvcChanged: (val) => setState(() => _avcAllowed = val),
                                av1Allowed: _av1Allowed,
                                onAv1Changed: (val) => setState(() => _av1Allowed = val),
                                mpegAllowed: _mpegAllowed,
                                onMpegChanged: (val) => setState(() => _mpegAllowed = val),
                                flacAudio: _flacAudio,
                                onFlacChanged: (val) => setState(() => _flacAudio = val),
                                opusAudio: _opusAudio,
                                onOpusChanged: (val) => setState(() => _opusAudio = val),
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 16),
                        Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Expanded(
                              flex: 6,
                              child: RulesSwarmHealthCard(
                                autoSeedController: _autoSeedController,
                                triageMinController: _triageMinController,
                                triageMaxController: _triageMaxController,
                                bypassInboxStagnation: _bypassInboxStagnation,
                                onBypassChanged: (val) => setState(() => _bypassInboxStagnation = val),
                              ),
                            ),
                            const SizedBox(width: 16),
                            Expanded(
                              flex: 6,
                              child: RulesBlacklistCard(
                                blacklistInputController: _blacklistInputController,
                                blacklistItems: _blacklistItems,
                                onBlacklistChanged: (val) => setState(() => _blacklistItems = val),
                              ),
                            ),
                          ],
                        ),
                      ],
                    );
                  } else {
                    return Column(
                      children: [
                        RulesFansubGroupsCard(
                          fansubGroups: _fansubGroups,
                          onGroupsChanged: (val) => setState(() => _fansubGroups = val),
                        ),
                        const SizedBox(height: 16),
                        RulesQualityCodecCard(
                          targetResolution: _targetResolution,
                          onResolutionChanged: (val) => setState(() => _targetResolution = val),
                          hevcAllowed: _hevcAllowed,
                          onHevcChanged: (val) => setState(() => _hevcAllowed = val),
                          avcAllowed: _avcAllowed,
                          onAvcChanged: (val) => setState(() => _avcAllowed = val),
                          av1Allowed: _av1Allowed,
                          onAv1Changed: (val) => setState(() => _av1Allowed = val),
                          mpegAllowed: _mpegAllowed,
                          onMpegChanged: (val) => setState(() => _mpegAllowed = val),
                          flacAudio: _flacAudio,
                          onFlacChanged: (val) => setState(() => _flacAudio = val),
                          opusAudio: _opusAudio,
                          onOpusChanged: (val) => setState(() => _opusAudio = val),
                        ),
                        const SizedBox(height: 16),
                        RulesSwarmHealthCard(
                          autoSeedController: _autoSeedController,
                          triageMinController: _triageMinController,
                          triageMaxController: _triageMaxController,
                          bypassInboxStagnation: _bypassInboxStagnation,
                          onBypassChanged: (val) => setState(() => _bypassInboxStagnation = val),
                        ),
                        const SizedBox(height: 16),
                        RulesBlacklistCard(
                          blacklistInputController: _blacklistInputController,
                          blacklistItems: _blacklistItems,
                          onBlacklistChanged: (val) => setState(() => _blacklistItems = val),
                        ),
                      ],
                    );
                  }
                },
              ),

              const SizedBox(height: 20),
              // Policy Matcher Telemetry Simulator
              RulesTelemetrySimulator(state: state),
            ],
          ),
        ),
      ),
    );
  }
}
