import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models/indexer_item.dart';
import '../state/indexer_cubit.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import 'cyber_badge.dart';
import 'loading/inline_spinner.dart';

class IndexerSettingsCard extends StatefulWidget {
  const IndexerSettingsCard({super.key});

  @override
  State<IndexerSettingsCard> createState() => _IndexerSettingsCardState();
}

class _IndexerSettingsCardState extends State<IndexerSettingsCard> {
  bool _showAddForm = false;
  final _idController = TextEditingController();
  final _nameController = TextEditingController();
  final _urlController = TextEditingController();
  final _apiKeyController = TextEditingController();

  bool _includeMovie = true;
  bool _includeSeries = true;
  bool _isSaving = false;

  @override
  void dispose() {
    _idController.dispose();
    _nameController.dispose();
    _urlController.dispose();
    _apiKeyController.dispose();
    super.dispose();
  }

  void _resetForm() {
    _idController.clear();
    _nameController.clear();
    _urlController.clear();
    _apiKeyController.clear();
    setState(() {
      _showAddForm = false;
      _includeMovie = true;
      _includeSeries = true;
    });
  }

  Future<void> _submitAdd() async {
    final name = _nameController.text.trim();
    final url = _urlController.text.trim();
    final key = _apiKeyController.text.trim();
    var id = _idController.text.trim();
    if (id.isEmpty) {
      id = name.toLowerCase().replaceAll(RegExp(r'[^a-z0-9_]'), '_');
    }

    if (id.isEmpty || name.isEmpty || url.isEmpty || key.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Please fill out all required indexer fields'),
          backgroundColor: ObsidianColors.error,
        ),
      );
      return;
    }

    final kinds = <String>[];
    if (_includeMovie) kinds.add('movie');
    if (_includeSeries) kinds.add('series');

    setState(() => _isSaving = true);
    final ok = await context.read<IndexerCubit>().addIndexer(
      id: id,
      name: name,
      url: url,
      apiKey: key,
      kinds: kinds,
      test: true,
    );

    // Immediate security hygiene: wipe text controller
    _apiKeyController.clear();

    if (!mounted) return;
    setState(() => _isSaving = false);

    if (ok) {
      _resetForm();
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Indexer "$name" added and tested'),
          backgroundColor: ObsidianColors.tertiary,
        ),
      );
    } else {
      final err = context.read<IndexerCubit>().state.indexersError;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(err ?? 'Failed to add indexer'),
          backgroundColor: ObsidianColors.error,
        ),
      );
    }
  }

  Future<void> _confirmRemove(IndexerSummary idx) async {
    final confirm = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: ObsidianColors.surfaceContainerLow,
        title: Text('Remove Indexer', style: ObsidianTypography.headlineMd()),
        content: Text(
          'Are you sure you want to remove "${idx.name}" (${idx.id})? Cached capabilities and credentials will be deleted.',
          style: ObsidianTypography.bodyMd(),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            style: ElevatedButton.styleFrom(
              backgroundColor: ObsidianColors.error,
              foregroundColor: ObsidianColors.onError,
            ),
            onPressed: () => Navigator.of(ctx).pop(true),
            child: const Text('Remove'),
          ),
        ],
      ),
    );

    if (confirm == true && mounted) {
      await context.read<IndexerCubit>().removeIndexer(idx.id);
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<IndexerCubit>().state;
    final indexers = state.indexers;

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
          // Header
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Row(
                children: [
                  Container(
                    padding: const EdgeInsets.all(6),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerHigh,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: const Icon(
                      Icons.rss_feed_rounded,
                      size: 20,
                      color: ObsidianColors.primary,
                    ),
                  ),
                  const SizedBox(width: 10),
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Torznab Indexers',
                        style: ObsidianTypography.titleSm(),
                      ),
                      Text(
                        'Prowlarr, Jackett, or standalone Torznab endpoints',
                        style: ObsidianTypography.bodySm(
                          color: ObsidianColors.outline,
                        ),
                      ),
                    ],
                  ),
                ],
              ),
              ElevatedButton.icon(
                icon: Icon(_showAddForm ? Icons.close : Icons.add, size: 16),
                label: Text(_showAddForm ? 'Cancel' : 'Add Indexer'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: _showAddForm
                      ? ObsidianColors.surfaceContainerHigh
                      : ObsidianColors.primary,
                  foregroundColor: _showAddForm
                      ? ObsidianColors.onSurface
                      : ObsidianColors.onPrimary,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                ),
                onPressed: () {
                  setState(() => _showAddForm = !_showAddForm);
                  if (!_showAddForm) _resetForm();
                },
              ),
            ],
          ),
          const SizedBox(height: 16),

          // Add Form
          if (_showAddForm) ...[
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerHigh,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: ObsidianColors.border),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'ADD NEW TORZNAB INDEXER',
                    style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.primary,
                    ),
                  ),
                  const SizedBox(height: 12),
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: _nameController,
                          style: ObsidianTypography.bodyMd(),
                          decoration: const InputDecoration(
                            labelText: 'Indexer Name (e.g. Prowlarr Movies)',
                          ),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: TextField(
                          controller: _idController,
                          style: ObsidianTypography.bodyMd(),
                          decoration: const InputDecoration(
                            labelText: 'Unique ID (optional, auto-generated)',
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: _urlController,
                    style: ObsidianTypography.bodyMd(),
                    decoration: const InputDecoration(
                      labelText:
                          'Torznab Base URL (e.g. http://127.0.0.1:9696/1/api)',
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: _apiKeyController,
                    obscureText: true,
                    enableSuggestions: false,
                    autocorrect: false,
                    enableInteractiveSelection: false,
                    style: ObsidianTypography.bodyMd(),
                    decoration: const InputDecoration(
                      labelText: 'API Key (masked, sent via stdin)',
                    ),
                  ),
                  const SizedBox(height: 12),
                  Row(
                    children: [
                      Text('Media Kinds: ', style: ObsidianTypography.bodySm()),
                      const SizedBox(width: 8),
                      FilterChip(
                        label: const Text('Movies'),
                        selected: _includeMovie,
                        onSelected: (v) => setState(() => _includeMovie = v),
                      ),
                      const SizedBox(width: 8),
                      FilterChip(
                        label: const Text('Series'),
                        selected: _includeSeries,
                        onSelected: (v) => setState(() => _includeSeries = v),
                      ),
                      const Spacer(),
                      ElevatedButton(
                        onPressed: _isSaving ? null : _submitAdd,
                        style: ElevatedButton.styleFrom(
                          backgroundColor: ObsidianColors.primary,
                          foregroundColor: ObsidianColors.onPrimary,
                        ),
                        child: _isSaving
                            ? const InlineSpinner(size: 16)
                            : const Text('Test & Save'),
                      ),
                    ],
                  ),
                ],
              ),
            ),
            const SizedBox(height: 16),
          ],

          // Indexers List
          if (state.isLoadingIndexers)
            const Center(child: InlineSpinner(size: 24))
          else if (indexers.isEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 12.0),
              child: Text(
                'No Torznab indexers configured yet. Add one above to search releases.',
                style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
              ),
            )
          else
            ListView.separated(
              shrinkWrap: true,
              physics: const NeverScrollableScrollPhysics(),
              itemCount: indexers.length,
              separatorBuilder: (_, __) =>
                  const Divider(color: ObsidianColors.border),
              itemBuilder: (context, index) {
                final idx = indexers[index];
                final isTestingThis =
                    state.isTestingIndexer && state.testingIndexerId == idx.id;

                return ListTile(
                  contentPadding: EdgeInsets.zero,
                  leading: Switch(
                    value: idx.enabled,
                    activeColor: ObsidianColors.primary,
                    onChanged: (val) {
                      context.read<IndexerCubit>().setIndexerEnabled(
                        idx.id,
                        val,
                      );
                    },
                  ),
                  title: Row(
                    children: [
                      Text(idx.name, style: ObsidianTypography.titleSm()),
                      const SizedBox(width: 8),
                      Text(
                        '(${idx.id})',
                        style: ObsidianTypography.monoCode(
                          color: ObsidianColors.outline,
                        ),
                      ),
                      const SizedBox(width: 8),
                      if (idx.lastTestStatus != null)
                        CyberBadge(
                          label: idx.lastTestStatus!,
                          customColor: idx.isHealthy
                              ? ObsidianColors.tertiary
                              : ObsidianColors.error,
                        ),
                    ],
                  ),
                  subtitle: Text(
                    '${idx.baseUrl} · [${idx.kinds.join(', ')}]',
                    style: ObsidianTypography.bodySm(
                      color: ObsidianColors.onSurfaceVariant,
                    ),
                  ),
                  trailing: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Tooltip(
                        message: 'Test indexer connection and capabilities',
                        child: OutlinedButton(
                          onPressed: isTestingThis
                              ? null
                              : () => context.read<IndexerCubit>().testIndexer(
                                  idx.id,
                                ),
                          child: isTestingThis
                              ? const InlineSpinner(size: 14)
                              : const Text('Test'),
                        ),
                      ),
                      const SizedBox(width: 8),
                      IconButton(
                        tooltip: 'Remove indexer',
                        icon: const Icon(
                          Icons.delete_outline,
                          color: ObsidianColors.error,
                          size: 20,
                        ),
                        onPressed: () => _confirmRemove(idx),
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
