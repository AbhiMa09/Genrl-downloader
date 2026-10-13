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
  // Global rule controllers
  final _groupsController = TextEditingController();
  final _blockedUploadersController = TextEditingController();
  final _seedAutoController = TextEditingController();
  final _seedAskMinController = TextEditingController();
  final _seedAskMaxController = TextEditingController();
  final _ratioTargetController = TextEditingController();

  String _preferredResolution = '1080p';
  String _minResolution = '720p';
  String _moveMode = 'hardlink';
  bool _autoPlanToWatch = false;

  bool _loaded = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<StateProvider>().refreshRules();
    });
  }

  void _populateFromRule(RuleItem rule) {
    _groupsController.text = rule.groups.join(', ');
    _blockedUploadersController.text = rule.blockedUploaders.join(', ');
    _seedAutoController.text = rule.seedAutoThreshold.toString();
    _seedAskMinController.text = rule.seedAskMin.toString();
    _seedAskMaxController.text = rule.seedAskMax.toString();
    _ratioTargetController.text = (rule.seedRatioTarget ?? 1.0).toString();
    _preferredResolution = rule.preferredResolution;
    _minResolution = rule.minResolution;
    _moveMode = rule.moveMode ?? 'hardlink';
    _autoPlanToWatch = rule.autoPlanToWatch ?? false;
    _loaded = true;
  }

  @override
  void dispose() {
    _groupsController.dispose();
    _blockedUploadersController.dispose();
    _seedAutoController.dispose();
    _seedAskMinController.dispose();
    _seedAskMaxController.dispose();
    _ratioTargetController.dispose();
    super.dispose();
  }

  Future<void> _saveGlobalRules(StateProvider state) async {
    final groups = _groupsController.text
        .split(',')
        .map((s) => s.trim())
        .filter((s) => s.isNotEmpty)
        .toList();
    final blocked = _blockedUploadersController.text
        .split(',')
        .map((s) => s.trim())
        .filter((s) => s.isNotEmpty)
        .toList();
    final seedAuto = int.tryParse(_seedAutoController.text.trim()) ?? 10;
    final askMin = int.tryParse(_seedAskMinController.text.trim()) ?? 3;
    final askMax = int.tryParse(_seedAskMaxController.text.trim()) ?? 9;
    final ratio = double.tryParse(_ratioTargetController.text.trim()) ?? 1.0;

    await state.updateRule('groups', groups);
    await state.updateRule('preferred_resolution', _preferredResolution);
    await state.updateRule('min_resolution', _minResolution);
    await state.updateRule('blocked_uploaders', blocked);
    await state.updateRule('seed_auto_threshold', seedAuto);
    await state.updateRule('seed_ask_min', askMin);
    await state.updateRule('seed_ask_max', askMax);
    await state.updateRule('move_mode', _moveMode);
    await state.updateRule('seed_ratio_target', ratio);
    await state.updateRule('auto_plan_to_watch', _autoPlanToWatch);

    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Global rules saved successfully'),
          backgroundColor: ObsidianColors.success,
        ),
      );
    }
  }

  Future<void> _resetToDefaults(StateProvider state) async {
    await state.resetRule();
    _loaded = false;
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Rules reset to default values'),
          backgroundColor: ObsidianColors.tertiary,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    final globalRule = state.rules.firstWhere(
      (r) => r.isGlobal,
      orElse: () => const RuleItem(
        id: 0,
        showTitle: 'Global Defaults',
        isGlobal: true,
      ),
    );

    if (!_loaded) {
      _populateFromRule(globalRule);
    }

    final showOverrides = state.rules.where((r) => !r.isGlobal).toList();

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Text('Matching Rules & Preferences', style: ObsidianTypography.headlineLg()),
              const Spacer(),
              OutlinedButton.icon(
                onPressed: () => _resetToDefaults(state),
                icon: const Icon(Icons.restore_rounded, size: 16),
                label: const Text('Reset to Defaults'),
                style: OutlinedButton.styleFrom(
                  side: const BorderSide(color: ObsidianColors.border),
                  foregroundColor: ObsidianColors.outline,
                ),
              ),
              const SizedBox(width: 10),
              ElevatedButton.icon(
                onPressed: () => _saveGlobalRules(state),
                icon: const Icon(Icons.save_rounded, size: 16),
                label: const Text('Save Rules'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.primaryContainer,
                  foregroundColor: ObsidianColors.onPrimaryContainer,
                  elevation: 0,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            'Global matching defaults govern fansub groups, resolutions, seed thresholds and library filing. Overrides can be attached per show.',
            style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
          ),
          const SizedBox(height: 24),

          if (state.rulesError != null)
            ErrorStateView(
              title: 'Rules Fetch Error',
              message: state.rulesError!,
              onRetry: () => state.refreshRules(),
              retryLabel: 'Retry Rules Query',
            )
          else if (state.isLoadingRules && state.rules.isEmpty)
            const LoadingStateView(message: 'Querying matching rules and thresholds from core...')
          else ...[
          // 1. EDITABLE GLOBAL RULES CARD
          Container(
            padding: const EdgeInsets.all(20),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLow,
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: ObsidianColors.border),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                      decoration: BoxDecoration(
                        color: ObsidianColors.tertiary.withOpacity(0.15),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        'GLOBAL RULES',
                        style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Text('Default Download Criteria', style: ObsidianTypography.titleSm()),
                  ],
                ),
                const SizedBox(height: 16),

                // Preferred Fansub Groups
                _TextFieldRow(
                  label: 'Preferred Fansub Groups (comma separated, ordered by priority)',
                  controller: _groupsController,
                  hintText: 'SubsPlease, Erai-raws, Judas',
                ),
                const SizedBox(height: 14),

                // Resolutions (Preferred & Minimum Floor)
                Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('Preferred Resolution', style: ObsidianTypography.bodySm()),
                          const SizedBox(height: 6),
                          _DropdownSelector(
                            value: _preferredResolution,
                            items: const ['2160p', '1080p', '720p', '480p'],
                            onChanged: (v) {
                              if (v != null) setState(() => _preferredResolution = v);
                            },
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('Minimum Resolution (Hard Floor)', style: ObsidianTypography.bodySm()),
                          const SizedBox(height: 6),
                          _DropdownSelector(
                            value: _minResolution,
                            items: const ['1080p', '720p', '480p'],
                            onChanged: (v) {
                              if (v != null) setState(() => _minResolution = v);
                            },
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 14),

                // Blocked Uploaders
                _TextFieldRow(
                  label: 'Blocked Uploaders (never downloaded)',
                  controller: _blockedUploadersController,
                  hintText: 'BadUploader1, ScamGroup',
                ),
                const SizedBox(height: 14),

                // Seed Thresholds: Auto >= N, Ask range
                Row(
                  children: [
                    Expanded(
                      child: _TextFieldRow(
                        label: 'Auto-download seeders (>= N)',
                        controller: _seedAutoController,
                        hintText: '10',
                      ),
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: _TextFieldRow(
                        label: 'Ask min seeders',
                        controller: _seedAskMinController,
                        hintText: '3',
                      ),
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: _TextFieldRow(
                        label: 'Ask max seeders',
                        controller: _seedAskMaxController,
                        hintText: '9',
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 14),

                // Library Filing Mode & Seeding Ratio Target
                Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('Library Ingestion (Move Mode)', style: ObsidianTypography.bodySm()),
                          const SizedBox(height: 6),
                          _DropdownSelector(
                            value: _moveMode,
                            items: const ['hardlink', 'move', 'copy'],
                            onChanged: (v) {
                              if (v != null) setState(() => _moveMode = v);
                            },
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: _TextFieldRow(
                        label: 'Keep until ratio (Target)',
                        controller: _ratioTargetController,
                        hintText: '1.0',
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 14),

                // Auto Plan to Watch Switch
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text('Auto-download "Plan to Watch" shows', style: ObsidianTypography.bodySm()),
                        Text(
                          'If disabled, Plan to Watch shows require manual approval before downloading.',
                          style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                        ),
                      ],
                    ),
                    Switch(
                      value: _autoPlanToWatch,
                      onChanged: (v) => setState(() => _autoPlanToWatch = v),
                      activeColor: ObsidianColors.primary,
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // 2. PER-SHOW OVERRIDES LIST
          Text('PER-SHOW OVERRIDES (${showOverrides.length})', style: ObsidianTypography.titleSm()),
          const SizedBox(height: 12),

          if (showOverrides.isEmpty)
            Container(
              padding: const EdgeInsets.all(24),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLowest,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: ObsidianColors.border),
              ),
              alignment: Alignment.center,
              child: Text(
                'No per-show overrides yet. You can customize rules for specific anime in Show Detail.',
                style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
              ),
            )
          else
            ...showOverrides.map((rule) {
              return Container(
                margin: const EdgeInsets.only(bottom: 12),
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLow,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Container(
                          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                          decoration: BoxDecoration(
                            color: ObsidianColors.primary.withOpacity(0.15),
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Text(
                            'SHOW #${rule.malId}',
                            style: ObsidianTypography.labelBadge(color: ObsidianColors.primary),
                          ),
                        ),
                        const SizedBox(width: 10),
                        Text(rule.showTitle, style: ObsidianTypography.titleSm()),
                        const Spacer(),
                        IconButton(
                          onPressed: () => state.resetRule(malId: rule.malId),
                          tooltip: 'Remove Override',
                          icon: const Icon(Icons.delete_outline_rounded, size: 16, color: ObsidianColors.error),
                        ),
                      ],
                    ),
                    const SizedBox(height: 10),
                    Wrap(
                      spacing: 20,
                      runSpacing: 8,
                      children: [
                        _RuleAttribute(label: 'Preferred Groups', value: rule.preferredGroup),
                        _RuleAttribute(label: 'Resolution', value: rule.preferredResolution),
                        _RuleAttribute(label: 'Min Seeders', value: '≥ ${rule.seedAutoThreshold}'),
                        _RuleAttribute(label: 'Move Mode', value: rule.moveMode ?? 'hardlink'),
                      ],
                    ),
                  ],
                ),
              );
            }),
          ],
        ],
      ),
    );
  }
}

extension _IterableExt<T> on Iterable<T> {
  Iterable<T> filter(bool Function(T) predicate) => where(predicate);
}

class _DropdownSelector extends StatelessWidget {
  final String value;
  final List<String> items;
  final ValueChanged<String?> onChanged;

  const _DropdownSelector({
    required this.value,
    required this.items,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 38,
      padding: const EdgeInsets.symmetric(horizontal: 12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: DropdownButtonHideUnderline(
        child: DropdownButton<String>(
          value: items.contains(value) ? value : items.first,
          dropdownColor: ObsidianColors.surfaceContainerHigh,
          isDense: true,
          items: items.map((i) {
            return DropdownMenuItem(
              value: i,
              child: Text(i, style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface)),
            );
          }).toList(),
          onChanged: onChanged,
        ),
      ),
    );
  }
}

class _RuleAttribute extends StatelessWidget {
  final String label;
  final String value;

  const _RuleAttribute({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: ObsidianTypography.bodySm(color: ObsidianColors.outline)),
        const SizedBox(height: 2),
        Text(value, style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface)),
      ],
    );
  }
}

class _TextFieldRow extends StatelessWidget {
  final String label;
  final TextEditingController controller;
  final String hintText;

  const _TextFieldRow({
    required this.label,
    required this.controller,
    required this.hintText,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: ObsidianTypography.bodySm()),
        const SizedBox(height: 6),
        Container(
          height: 38,
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainerLowest,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: ObsidianColors.border),
          ),
          padding: const EdgeInsets.symmetric(horizontal: 12),
          alignment: Alignment.centerLeft,
          child: TextField(
            controller: controller,
            style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
            decoration: InputDecoration(
              hintText: hintText,
              hintStyle: ObsidianTypography.monoCode(color: ObsidianColors.outline),
              border: InputBorder.none,
              isDense: true,
              contentPadding: EdgeInsets.zero,
            ),
          ),
        ),
      ],
    );
  }
}
