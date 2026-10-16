import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import 'accounts_section.dart';
import 'engine_section.dart';
import 'folders_section.dart';
import 'qbittorrent_section.dart';
import 'settings_command_strip.dart';
import 'settings_nav_tree.dart';

class SettingsPage extends StatefulWidget {
  final bool isGoldenTest;
  final String? legacySecondaryTitle;
  final String? legacySecondarySubtitle;
  final String? legacySecondaryButton;

  const SettingsPage({
    super.key,
    this.isGoldenTest = false,
    this.legacySecondaryTitle,
    this.legacySecondarySubtitle,
    this.legacySecondaryButton,
  });

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  // Navigation Sub-tab
  int _activeSubtabIndex = 0; // 0: Accounts, 1: qBittorrent, 2: Folders, 3: Downloads, 4: Rate limits, 5: Appearance, 6: Notifications, 7: Backup

  // Input Controllers
  final _qbitUrlController = TextEditingController(
    text: 'http://127.0.0.1:8080',
  );
  final _qbitCategoryController = TextEditingController(
    text: 'anime-automated',
  );
  final _qbitUserController = TextEditingController(text: 'admin');
  final _qbitPasswordController = TextEditingController(
    text: 'super_secure_qbittorrent_token',
  );

  final _libraryFolderController = TextEditingController(text: '/mnt/anime/TV');
  final _downloadFolderController = TextEditingController(
    text: '/mnt/nvme_pool/torrents/completed',
  );
  final _namingPatternController = TextEditingController(
    text: '{Series Title}/Season {Season:02d}/{Series Title} - S{Season:02d}E{Episode:02d} [{Resolution}] [{Group}].{Ext}',
  );

  String _ingestionStrategy = 'hardlink'; // hardlink, copy, move
  int _concurrency = 2;
  int _dailyCap = 30;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final state = context.read<StateProvider>();
      state.refreshMalStatus();
      state.refreshLocations().then((_) {
        if (state.locationsReport != null && mounted) {
          final dl = state.downloadCheck?['path'];
          final lib = state.libraryCheck?['path'];
          if (dl != null && dl is String && dl.isNotEmpty) {
            _downloadFolderController.text = dl;
          }
          if (lib != null && lib is String && lib.isNotEmpty) {
            _libraryFolderController.text = lib;
          }
        }
      });
    });
  }

  @override
  void dispose() {
    _qbitUrlController.dispose();
    _qbitCategoryController.dispose();
    _qbitUserController.dispose();
    _qbitPasswordController.dispose();
    _libraryFolderController.dispose();
    _downloadFolderController.dispose();
    _namingPatternController.dispose();
    super.dispose();
  }

  Future<void> _savePreferences(StateProvider state) async {
    // Save to state provider
    await state.updateLocations(
      downloadDir: _downloadFolderController.text.trim(),
      libraryDir: _libraryFolderController.text.trim(),
    );
    await state.updateRule('daily_cap', _dailyCap);
    await state.updateRule('concurrency', _concurrency);
    await state.updateRule('move_mode', _ingestionStrategy);

    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Preferences saved to /etc/genrl/core.yaml'),
          backgroundColor: ObsidianColors.success,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 20),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 1440),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // Top Command Strip & Breadcrumbs
              SettingsCommandStrip(
                state: state,
                onSave: () => _savePreferences(state),
              ),
              const SizedBox(height: 20),

              // Sub-nav rail + Main panels
              LayoutBuilder(
                builder: (context, constraints) {
                  final isWide = constraints.maxWidth > 900;
                  if (isWide) {
                    return Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        SizedBox(
                          width: 220,
                          child: SettingsNavTree(
                            activeIndex: _activeSubtabIndex,
                            onSelect: (idx) =>
                                setState(() => _activeSubtabIndex = idx),
                            state: state,
                          ),
                        ),
                        const SizedBox(width: 20),
                        Expanded(child: _buildSettingsPanels(context, state)),
                      ],
                    );
                  } else {
                    return Column(
                      children: [
                        SettingsNavTree(
                          activeIndex: _activeSubtabIndex,
                          onSelect: (idx) =>
                              setState(() => _activeSubtabIndex = idx),
                          state: state,
                        ),
                        const SizedBox(height: 16),
                        _buildSettingsPanels(context, state),
                      ],
                    );
                  }
                },
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildSettingsPanels(BuildContext context, StateProvider state) {
    return Column(
      children: [
        // SECTION B: Tracker & Indexer Accounts
        SettingsAccountsSection(
          state: state,
          isGoldenTest: widget.isGoldenTest,
          legacySecondaryTitle: widget.legacySecondaryTitle,
          legacySecondarySubtitle: widget.legacySecondarySubtitle,
          legacySecondaryButton: widget.legacySecondaryButton,
        ),
        const SizedBox(height: 16),

        // SECTION C: qBittorrent Client Integration Card
        SettingsQbittorrentSection(
          state: state,
          urlController: _qbitUrlController,
          categoryController: _qbitCategoryController,
          userController: _qbitUserController,
          passwordController: _qbitPasswordController,
        ),
        const SizedBox(height: 16),

        // SECTION D: Folders & Filesystem Ingestion Card
        SettingsFoldersSection(
          state: state,
          libraryFolderController: _libraryFolderController,
          downloadFolderController: _downloadFolderController,
          namingPatternController: _namingPatternController,
          ingestionStrategy: _ingestionStrategy,
          onIngestionStrategyChanged: (val) =>
              setState(() => _ingestionStrategy = val),
        ),
        const SizedBox(height: 16),

        // SECTION E: Download Engine & Concurrency Card
        SettingsEngineSection(
          state: state,
          concurrency: _concurrency,
          onConcurrencyChanged: (val) => setState(() => _concurrency = val),
          dailyCap: _dailyCap,
          onDailyCapChanged: (val) => setState(() => _dailyCap = val),
        ),
      ],
    );
  }
}
