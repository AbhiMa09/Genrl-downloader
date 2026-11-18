import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../state/indexer_cubit.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import 'cyber_badge.dart';
import 'loading/inline_spinner.dart';

class TmdbSettingsCard extends StatefulWidget {
  const TmdbSettingsCard({super.key});

  @override
  State<TmdbSettingsCard> createState() => _TmdbSettingsCardState();
}

class _TmdbSettingsCardState extends State<TmdbSettingsCard> {
  final _keyController = TextEditingController();
  bool _isSaving = false;

  @override
  void dispose() {
    _keyController.dispose();
    super.dispose();
  }

  Future<void> _handleSave() async {
    final key = _keyController.text.trim();
    if (key.isEmpty) return;

    setState(() => _isSaving = true);
    final ok = await context.read<IndexerCubit>().setTmdbKey(key);
    // Explicitly wipe the text controller immediately after submit per security rule
    _keyController.clear();

    if (!mounted) return;
    setState(() => _isSaving = false);

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          ok ? 'TMDB API key saved securely' : 'Failed to save TMDB API key',
        ),
        backgroundColor: ok ? ObsidianColors.tertiary : ObsidianColors.error,
      ),
    );
  }

  Future<void> _handleTest() async {
    final ok = await context.read<IndexerCubit>().testTmdb();
    if (!mounted) return;

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          ok ? 'TMDB connection test passed' : 'TMDB connection test failed',
        ),
        backgroundColor: ok ? ObsidianColors.tertiary : ObsidianColors.error,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<IndexerCubit>().state;
    final isConfigured = state.isTmdbConfigured;
    final isTesting = state.isTestingTmdb;

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
                      Icons.movie_filter_rounded,
                      size: 20,
                      color: ObsidianColors.primary,
                    ),
                  ),
                  const SizedBox(width: 10),
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'TMDB Metadata Integration',
                        style: ObsidianTypography.titleSm(),
                      ),
                      Text(
                        'Search movies and series metadata via The Movie Database',
                        style: ObsidianTypography.bodySm(
                          color: ObsidianColors.outline,
                        ),
                      ),
                    ],
                  ),
                ],
              ),
              CyberBadge(
                label: isConfigured ? 'Configured' : 'Not Configured',
                customColor: isConfigured
                    ? ObsidianColors.tertiary
                    : ObsidianColors.outline,
              ),
            ],
          ),
          const SizedBox(height: 16),

          // Obscured API Key Field (strictly no autofill, no copy button)
          Text(
            'API Key (Read Access Token or v3 Key)',
            style: ObsidianTypography.labelBadge(
              color: ObsidianColors.onSurface,
            ),
          ),
          const SizedBox(height: 6),
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _keyController,
                  obscureText: true,
                  enableSuggestions: false,
                  autocorrect: false,
                  enableInteractiveSelection: false,
                  style: ObsidianTypography.bodyMd(),
                  decoration: InputDecoration(
                    hintText: isConfigured
                        ? '••••••••••••••••••••••••••••••••'
                        : 'Enter your TMDB API key...',
                    hintStyle: ObsidianTypography.bodyMd(
                      color: ObsidianColors.outline,
                    ),
                    filled: true,
                    fillColor: ObsidianColors.surfaceContainerHigh,
                    contentPadding: const EdgeInsets.symmetric(
                      horizontal: 14,
                      vertical: 10,
                    ),
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: const BorderSide(
                        color: ObsidianColors.border,
                      ),
                    ),
                    enabledBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: const BorderSide(
                        color: ObsidianColors.border,
                      ),
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 10),
              ElevatedButton(
                onPressed: _isSaving ? null : _handleSave,
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.primary,
                  foregroundColor: ObsidianColors.onPrimary,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 18,
                    vertical: 14,
                  ),
                ),
                child: _isSaving
                    ? const InlineSpinner(size: 16)
                    : const Text('Save Key'),
              ),
              if (isConfigured) ...[
                const SizedBox(width: 8),
                OutlinedButton(
                  onPressed: isTesting ? null : _handleTest,
                  style: OutlinedButton.styleFrom(
                    foregroundColor: ObsidianColors.onSurface,
                    side: const BorderSide(color: ObsidianColors.border),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                    padding: const EdgeInsets.symmetric(
                      horizontal: 16,
                      vertical: 14,
                    ),
                  ),
                  child: isTesting
                      ? const InlineSpinner(size: 16)
                      : const Text('Test Connection'),
                ),
              ],
            ],
          ),

          if (state.tmdbError != null) ...[
            const SizedBox(height: 8),
            Text(
              state.tmdbError!,
              style: ObsidianTypography.bodySm(color: ObsidianColors.error),
            ),
          ],

          const SizedBox(height: 16),
          const Divider(color: ObsidianColors.border),
          const SizedBox(height: 8),

          // Mandatory TMDB Attribution Requirement
          Text(
            'This product uses the TMDB API but is not endorsed or certified by TMDB.',
            style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
          ),
        ],
      ),
    );
  }
}
