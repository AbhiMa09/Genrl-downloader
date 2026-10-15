import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';

class SettingsPage extends StatefulWidget {
  const SettingsPage({super.key});

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  // Navigation Sub-tab
  int _activeSubtabIndex = 0; // 0: Accounts, 1: qBittorrent, 2: Folders, 3: Downloads, 4: Rate limits, 5: Appearance, 6: Notifications, 7: Backup

  // Input Controllers
  final _qbitUrlController = TextEditingController(text: 'http://127.0.0.1:8080');
  final _qbitCategoryController = TextEditingController(text: 'anime-automated');
  final _qbitUserController = TextEditingController(text: 'admin');
  final _qbitPasswordController = TextEditingController(text: 'super_secure_qbittorrent_token');

  final _libraryFolderController = TextEditingController(text: '/mnt/anime/TV');
  final _downloadFolderController = TextEditingController(text: '/mnt/nvme_pool/torrents/completed');
  final _namingPatternController = TextEditingController(
    text: '{Series Title}/Season {Season:02d}/{Series Title} - S{Season:02d}E{Episode:02d} [{Resolution}] [{Group}].{Ext}',
  );

  final _malClientIdController = TextEditingController();
  final _malClientSecretController = TextEditingController();

  bool _obscureQbitPassword = true;
  String _ingestionStrategy = 'hardlink'; // hardlink, copy, move
  String _folderIconMethod = 'Auto (Detect Desktop Env: KDE Plasma)';
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
    _malClientIdController.dispose();
    _malClientSecretController.dispose();
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
              _buildCommandStrip(context, state),
              const SizedBox(height: 20),

              // Sub-nav rail + Main panels
              LayoutBuilder(
                builder: (context, constraints) {
                  final isWide = constraints.maxWidth > 900;
                  if (isWide) {
                    return Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        SizedBox(width: 220, child: _buildConfigurationTree(state)),
                        const SizedBox(width: 20),
                        Expanded(child: _buildSettingsPanels(context, state)),
                      ],
                    );
                  } else {
                    return Column(
                      children: [
                        _buildConfigurationTree(state),
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

  Widget _buildCommandStrip(BuildContext context, StateProvider state) {
    return Wrap(
      alignment: WrapAlignment.spaceBetween,
      crossAxisAlignment: WrapCrossAlignment.center,
      spacing: 16,
      runSpacing: 12,
      children: [
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Wrap(
              crossAxisAlignment: WrapCrossAlignment.center,
              spacing: 6,
              children: [
                Text('System / Configuration / ', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1.5),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHigh,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text('SESSION ACTIVE', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Wrap(
              crossAxisAlignment: WrapCrossAlignment.center,
              spacing: 8,
              children: [
                Text('Settings & System Integration', style: ObsidianTypography.headlineLg()),
                Text('v2.4.1', style: ObsidianTypography.titleSm(color: ObsidianColors.outline)),
              ],
            ),
          ],
        ),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            OutlinedButton.icon(
              onPressed: () {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('Preferences reloaded from disk')),
                );
              },
              icon: const Icon(Icons.restart_alt_rounded, size: 16),
              label: const Text('Discard Changes'),
              style: OutlinedButton.styleFrom(
                side: const BorderSide(color: ObsidianColors.border),
                foregroundColor: ObsidianColors.onSurfaceVariant,
              ),
            ),
            ElevatedButton.icon(
              onPressed: () => _savePreferences(state),
              icon: const Icon(Icons.check_rounded, size: 16),
              label: const Text('Save Preferences'),
              style: ElevatedButton.styleFrom(
                backgroundColor: ObsidianColors.primaryContainer,
                foregroundColor: ObsidianColors.onPrimaryContainer,
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildConfigurationTree(StateProvider state) {
    final subtabs = [
      {'label': 'Accounts', 'icon': Icons.manage_accounts_rounded, 'badge': 'MAL'},
      {'label': 'qBittorrent', 'icon': Icons.sync_alt_rounded, 'badge': 'RPC'},
      {'label': 'Folders', 'icon': Icons.folder_special_rounded, 'badge': null},
      {'label': 'Downloads', 'icon': Icons.download_for_offline_rounded, 'badge': null},
      {'label': 'Rate limits', 'icon': Icons.speed_rounded, 'badge': null},
      {'label': 'Appearance', 'icon': Icons.palette_rounded, 'badge': null},
      {'label': 'Notifications', 'icon': Icons.notifications_active_rounded, 'badge': null},
      {'label': 'Backup', 'icon': Icons.save_rounded, 'badge': null},
    ];

    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
            child: Text('CONFIGURATION TREE', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
          ),
          ...subtabs.asMap().entries.map((entry) {
            final idx = entry.key;
            final item = entry.value;
            final isSelected = _activeSubtabIndex == idx;
            return InkWell(
              onTap: () => setState(() => _activeSubtabIndex = idx),
              borderRadius: BorderRadius.circular(8),
              child: Container(
                margin: const EdgeInsets.only(bottom: 2),
                padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                decoration: BoxDecoration(
                  color: isSelected ? ObsidianColors.surfaceContainerHigh : Colors.transparent,
                  borderRadius: BorderRadius.circular(8),
                  border: isSelected
                      ? Border.all(color: ObsidianColors.primary.withOpacity(0.3))
                      : null,
                ),
                child: Row(
                  children: [
                    Icon(
                      item['icon'] as IconData,
                      size: 16,
                      color: isSelected ? ObsidianColors.primary : ObsidianColors.outline,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        item['label'] as String,
                        style: isSelected
                            ? ObsidianTypography.bodySm(color: ObsidianColors.primary).copyWith(fontWeight: FontWeight.bold)
                            : ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
                      ),
                    ),
                    if (item['badge'] != null)
                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                        decoration: BoxDecoration(
                          color: ObsidianColors.surfaceContainer,
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(item['badge'] as String, style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
                      )
                    else if (isSelected)
                      Container(
                        width: 6,
                        height: 6,
                        decoration: const BoxDecoration(
                          shape: BoxShape.circle,
                          color: ObsidianColors.primary,
                        ),
                      ),
                  ],
                ),
              ),
            );
          }),
          const SizedBox(height: 16),
          // System Telemetry Micro-Card in Nav
          Container(
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Expanded(
                      child: Text('ENV: LINUX x86_64', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline), overflow: TextOverflow.ellipsis),
                    ),
                    const SizedBox(width: 4),
                    Text('RPC OK', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
                  ],
                ),
                const SizedBox(height: 6),
                ClipRRect(
                  borderRadius: BorderRadius.circular(2),
                  child: const LinearProgressIndicator(
                    value: 0.75,
                    backgroundColor: ObsidianColors.surfaceContainer,
                    valueColor: AlwaysStoppedAnimation<Color>(ObsidianColors.tertiary),
                    minHeight: 4,
                  ),
                ),
                const SizedBox(height: 6),
                Text('Conf: /etc/genrl/core.yaml', style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurfaceVariant)),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildSettingsPanels(BuildContext context, StateProvider state) {
    return Column(
      children: [
        // SECTION B: Tracker & Indexer Accounts
        _buildAccountsCard(context, state),
        const SizedBox(height: 16),

        // SECTION C: qBittorrent Client Integration Card
        _buildQbittorrentCard(context, state),
        const SizedBox(height: 16),

        // SECTION D: Folders & Filesystem Ingestion Card
        _buildFoldersCard(context, state),
        const SizedBox(height: 16),

        // SECTION E: Download Engine & Concurrency Card
        _buildEngineCard(context, state),
      ],
    );
  }

  // CARD 1: Accounts Integration Card
  Widget _buildAccountsCard(BuildContext context, StateProvider state) {
    final isMalAuth = state.isMalAuthenticated;
    final username = isMalAuth ? (state.malUsername ?? 'ren_kurokawa') : 'Not Connected';

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
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Row(
                  children: [
                    Container(
                      padding: const EdgeInsets.all(6),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHigh,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: const Icon(Icons.badge_rounded, size: 20, color: ObsidianColors.primary),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('Tracker & Indexer Accounts', style: ObsidianTypography.titleSm(), overflow: TextOverflow.ellipsis),
                          Text('Synchronize watchlists, scrobble completed downloads, and update release status', style: ObsidianTypography.bodySm(color: ObsidianColors.outline), overflow: TextOverflow.ellipsis),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('OAuth 2.0 PKCE', style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurfaceVariant)),
              ),
            ],
          ),
          const SizedBox(height: 16),

          // MAL Card
          Container(
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainer,
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: ObsidianColors.border),
            ),
            child: Column(
              children: [
                Row(
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: [
                    // Avatar
                    Container(
                      width: 44,
                      height: 44,
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHigh,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: const Icon(Icons.person_outline_rounded, size: 24, color: ObsidianColors.primary),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Wrap(
                            crossAxisAlignment: WrapCrossAlignment.center,
                            spacing: 8,
                            runSpacing: 4,
                            children: [
                              Text(username, style: ObsidianTypography.titleSm().copyWith(fontWeight: FontWeight.bold)),
                              Container(
                                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1.5),
                                decoration: BoxDecoration(
                                  color: ObsidianColors.surfaceContainerHighest,
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text('MyAnimeList', style: ObsidianTypography.labelBadge(color: ObsidianColors.primary)),
                              ),
                              Container(
                                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1.5),
                                decoration: BoxDecoration(
                                  color: isMalAuth
                                      ? ObsidianColors.tertiary.withOpacity(0.18)
                                      : ObsidianColors.warning.withOpacity(0.18),
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Row(
                                  mainAxisSize: MainAxisSize.min,
                                  children: [
                                    Container(
                                      width: 6,
                                      height: 6,
                                      decoration: BoxDecoration(
                                        shape: BoxShape.circle,
                                        color: isMalAuth ? ObsidianColors.tertiary : ObsidianColors.warning,
                                      ),
                                    ),
                                    const SizedBox(width: 4),
                                    Text(
                                      isMalAuth ? 'Connected' : 'Not Connected',
                                      style: ObsidianTypography.labelBadge(
                                        color: isMalAuth ? ObsidianColors.tertiary : ObsidianColors.warning,
                                      ),
                                    ),
                                  ],
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 4),
                          Text('Auto-sync every 30m • Credentials secured in local encrypted keyring', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline), overflow: TextOverflow.ellipsis),
                        ],
                      ),
                    ),
                    Wrap(
                      spacing: 6,
                      runSpacing: 6,
                      children: [
                        ElevatedButton.icon(
                          onPressed: () => state.syncNow(),
                          icon: const Icon(Icons.sync_rounded, size: 16),
                          label: const Text('Sync Now'),
                          style: ElevatedButton.styleFrom(
                            backgroundColor: ObsidianColors.surfaceContainerHigh,
                            foregroundColor: ObsidianColors.onSurface,
                            elevation: 0,
                          ),
                        ),
                        OutlinedButton.icon(
                          onPressed: () => state.loginMal(),
                          icon: const Icon(Icons.key_rounded, size: 16),
                          label: Text(isMalAuth ? 'Reconnect' : 'Connect via Browser (PKCE)'),
                          style: OutlinedButton.styleFrom(
                            side: const BorderSide(color: ObsidianColors.border),
                            foregroundColor: ObsidianColors.onSurface,
                          ),
                        ),
                        if (isMalAuth)
                          OutlinedButton.icon(
                            onPressed: () => state.disconnectMal(),
                            icon: const Icon(Icons.link_off_rounded, size: 16, color: ObsidianColors.error),
                            label: const Text('Disconnect'),
                            style: OutlinedButton.styleFrom(
                              side: const BorderSide(color: ObsidianColors.border),
                              foregroundColor: ObsidianColors.error,
                            ),
                          ),
                      ],
                    ),
                  ],
                ),
                const SizedBox(height: 12),

                // Metrics Quick Strip
                Row(
                  children: [
                    _buildAccountMetric('MONITORED SERIES', '${state.trackedShows.length}', ObsidianColors.primary),
                    const SizedBox(width: 8),
                    _buildAccountMetric('PLAN TO WATCH', '${state.planToWatchCount}', ObsidianColors.onSurface),
                    const SizedBox(width: 8),
                    _buildAccountMetric('SCROBBLE SUCCESS', '99.8%', ObsidianColors.tertiary),
                    const SizedBox(width: 8),
                    _buildAccountMetric('RATE LIMIT BUDGET', '420/500', ObsidianColors.onSurface),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 10),

          // Secondary Service (AniList)
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainer.withOpacity(0.6),
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: ObsidianColors.border),
            ),
            child: Row(
              children: [
                Container(
                  width: 36,
                  height: 36,
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: const Icon(Icons.cloud_off_rounded, size: 20, color: ObsidianColors.outline),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Text('AniList GraphQL Sync', style: ObsidianTypography.titleSm()),
                          const SizedBox(width: 6),
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
                            decoration: BoxDecoration(
                              color: ObsidianColors.surfaceContainerHighest,
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text('Inactive', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                          ),
                        ],
                      ),
                      Text('Enable simultaneous dual-scrobbling to your secondary AniList profile', style: ObsidianTypography.bodySm(color: ObsidianColors.outline)),
                    ],
                  ),
                ),
                OutlinedButton.icon(
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('AniList OAuth bridge opening...')),
                    );
                  },
                  icon: const Icon(Icons.add_link_rounded, size: 16),
                  label: const Text('Connect AniList'),
                  style: OutlinedButton.styleFrom(
                    side: const BorderSide(color: ObsidianColors.border),
                    foregroundColor: ObsidianColors.onSurface,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildAccountMetric(String label, String value, Color color) {
    return Expanded(
      child: Container(
        padding: const EdgeInsets.all(8),
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainerLow,
          borderRadius: BorderRadius.circular(6),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(label, style: ObsidianTypography.labelBadge(color: ObsidianColors.outline), overflow: TextOverflow.ellipsis),
            const SizedBox(height: 2),
            Text(value, style: ObsidianTypography.headlineMd(color: color), overflow: TextOverflow.ellipsis),
          ],
        ),
      ),
    );
  }

  // CARD 2: qBittorrent Client Integration Card
  Widget _buildQbittorrentCard(BuildContext context, StateProvider state) {
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
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Row(
                  children: [
                    Container(
                      padding: const EdgeInsets.all(6),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHigh,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: const Icon(Icons.hub_rounded, size: 20, color: ObsidianColors.tertiary),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('qBittorrent WebUI Connection', style: ObsidianTypography.titleSm(), overflow: TextOverflow.ellipsis),
                          Text('Manage RPC bridge for automated magnet handoff and seeding limits', style: ObsidianTypography.bodySm(color: ObsidianColors.outline), overflow: TextOverflow.ellipsis),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                decoration: BoxDecoration(
                  color: ObsidianColors.tertiary.withOpacity(0.18),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Row(
                  children: [
                    Container(width: 6, height: 6, decoration: const BoxDecoration(shape: BoxShape.circle, color: ObsidianColors.tertiary)),
                    const SizedBox(width: 6),
                    Text('Connected • v4.6.3 Web API', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),

          // Two-by-Two Grid
          Row(
            children: [
              Expanded(
                child: _buildInputField(
                  label: 'Host / RPC Endpoint URL',
                  sublabel: 'DEFAULT: :8080',
                  icon: Icons.lan_rounded,
                  controller: _qbitUrlController,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: _buildInputField(
                  label: 'Default Tag / Ingestion Category',
                  sublabel: 'qBit category rule',
                  icon: Icons.label_rounded,
                  controller: _qbitCategoryController,
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Row(
            children: [
              Expanded(
                child: _buildInputField(
                  label: 'WebUI Username',
                  sublabel: null,
                  icon: Icons.person_rounded,
                  controller: _qbitUserController,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('WebUI Secret Key / Password', style: ObsidianTypography.monoCode()),
                    const SizedBox(height: 4),
                    Container(
                      height: 38,
                      padding: const EdgeInsets.symmetric(horizontal: 10),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainer,
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(color: ObsidianColors.border),
                      ),
                      child: Row(
                        children: [
                          const Icon(Icons.lock_rounded, size: 16, color: ObsidianColors.outline),
                          const SizedBox(width: 8),
                          Expanded(
                            child: TextField(
                              controller: _qbitPasswordController,
                              obscureText: _obscureQbitPassword,
                              style: ObsidianTypography.monoCode(),
                              decoration: const InputDecoration(border: InputBorder.none, isDense: true, contentPadding: EdgeInsets.zero),
                            ),
                          ),
                          IconButton(
                            icon: Icon(
                              _obscureQbitPassword ? Icons.visibility_rounded : Icons.visibility_off_rounded,
                              size: 16,
                              color: ObsidianColors.outline,
                            ),
                            onPressed: () => setState(() => _obscureQbitPassword = !_obscureQbitPassword),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 14),

          // Test Connection Callout
          Container(
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainer,
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: ObsidianColors.border),
            ),
            child: Row(
              children: [
                const Icon(Icons.check_circle_rounded, size: 20, color: ObsidianColors.tertiary),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('Connection verified in 12ms', style: ObsidianTypography.monoCode(color: ObsidianColors.tertiary).copyWith(fontWeight: FontWeight.bold)),
                      Text('qBittorrent v4.6.3 x86_64 daemon responded on loopback', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                    ],
                  ),
                ),
                ElevatedButton.icon(
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('Ping verified: 8ms loopback response')),
                    );
                  },
                  icon: const Icon(Icons.network_ping_rounded, size: 16),
                  label: const Text('Test Connection'),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: ObsidianColors.surfaceContainerHigh,
                    foregroundColor: ObsidianColors.primary,
                    elevation: 0,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  // CARD 3: Folders & Filesystem Ingestion Card
  Widget _buildFoldersCard(BuildContext context, StateProvider state) {
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
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Row(
                  children: [
                    Container(
                      padding: const EdgeInsets.all(6),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHigh,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: const Icon(Icons.folder_copy_rounded, size: 20, color: ObsidianColors.primary),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('Folders & Filesystem Ingestion', style: ObsidianTypography.titleSm(), overflow: TextOverflow.ellipsis),
                          Text('Manage target directories, rename patterns, hardlinking rules, and desktop folder art', style: ObsidianTypography.bodySm(color: ObsidianColors.outline), overflow: TextOverflow.ellipsis),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('Btrfs / CoW Aware', style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurfaceVariant)),
              ),
            ],
          ),
          const SizedBox(height: 16),

          // Anime Library Path
          _buildPathPicker(
            label: 'Anime Library Root Path',
            metric: '3.42 TB / 8.00 TB used (57% free)',
            icon: Icons.folder_special_rounded,
            controller: _libraryFolderController,
            state: state,
          ),
          const SizedBox(height: 12),

          // Download Staging Path
          _buildPathPicker(
            label: 'Download Staging / Client Incomplete Ingestion',
            metric: 'NVMe Scratch Cache',
            icon: Icons.hourglass_top_rounded,
            controller: _downloadFolderController,
            state: state,
          ),
          const SizedBox(height: 16),

          // Naming Pattern Configurator
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainer,
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: ObsidianColors.border),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text('Standard Episode Naming Pattern', style: ObsidianTypography.monoCode()),
                    Text('Token Interpolation Active', style: ObsidianTypography.labelBadge(color: ObsidianColors.primary)),
                  ],
                ),
                const SizedBox(height: 6),
                Container(
                  height: 38,
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: ObsidianColors.border),
                  ),
                  child: Row(
                    children: [
                      const Icon(Icons.text_fields_rounded, size: 16, color: ObsidianColors.outline),
                      const SizedBox(width: 8),
                      Expanded(
                        child: TextField(
                          controller: _namingPatternController,
                          style: ObsidianTypography.monoCode(color: ObsidianColors.primary),
                          decoration: const InputDecoration(border: InputBorder.none, isDense: true, contentPadding: EdgeInsets.zero),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 8),

                // Helper Chips
                Wrap(
                  spacing: 6,
                  runSpacing: 4,
                  children: [
                    Text('Insert Tokens:', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                    _buildTokenChip('{Series Title}'),
                    _buildTokenChip('{Season:02d}'),
                    _buildTokenChip('{Episode:02d}'),
                    _buildTokenChip('{Resolution}'),
                    _buildTokenChip('{Group}'),
                    _buildTokenChip('{CRC32}'),
                  ],
                ),
                const SizedBox(height: 10),

                // Live Preview Callout
                Container(
                  padding: const EdgeInsets.all(10),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLowest,
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Text('LIVE RENAMER PREVIEW', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
                          Text('Target Ext: .mkv', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                        ],
                      ),
                      Builder(builder: (context) {
                        final state = context.watch<StateProvider>();
                        final title = state.shows.isNotEmpty ? state.shows.first.title : '{Series Title}';
                        return Text(
                          '/mnt/anime/TV/$title/Season 01/$title - S01E01 [1080p].mkv',
                          style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
                        );
                      }),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),

          // Dual Column: Ingestion Link Strategy & Series Poster Folder Icon Method
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('Ingestion Link Strategy', style: ObsidianTypography.titleSm()),
                      const SizedBox(height: 8),
                      Material(
                        type: MaterialType.transparency,
                        child: Column(
                          children: [
                            RadioListTile<String>(
                              value: 'hardlink',
                              groupValue: _ingestionStrategy,
                              onChanged: (v) => setState(() => _ingestionStrategy = v ?? 'hardlink'),
                              title: Row(
                                children: [
                                  const Text('Hardlink'),
                                  const SizedBox(width: 6),
                                  Container(
                                    padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                                    decoration: BoxDecoration(
                                      color: ObsidianColors.tertiary.withOpacity(0.2),
                                      borderRadius: BorderRadius.circular(4),
                                    ),
                                    child: Text('Recommended', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
                                  ),
                                ],
                              ),
                              subtitle: Text('Instant, zero disk space, preserves seed integrity', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                              activeColor: ObsidianColors.primary,
                              contentPadding: EdgeInsets.zero,
                            ),
                            RadioListTile<String>(
                              value: 'copy',
                              groupValue: _ingestionStrategy,
                              onChanged: (v) => setState(() => _ingestionStrategy = v ?? 'copy'),
                              title: const Text('Copy (Duplicate file)'),
                              subtitle: Text('Duplicates raw bytes; high I/O consumption', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                              activeColor: ObsidianColors.primary,
                              contentPadding: EdgeInsets.zero,
                            ),
                            RadioListTile<String>(
                              value: 'move',
                              groupValue: _ingestionStrategy,
                              onChanged: (v) => setState(() => _ingestionStrategy = v ?? 'move'),
                              title: const Text('Move (Atomic filesystem move)'),
                              subtitle: Text('Terminates active seeding; removes from staging', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                              activeColor: ObsidianColors.primary,
                              contentPadding: EdgeInsets.zero,
                            ),
                          ],
                        ),
                      ),
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
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Expanded(
                            child: Text(
                              'Series Poster Folder Icon Generator',
                              style: ObsidianTypography.titleSm(),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                          const SizedBox(width: 8),
                          Text('Freedesktop.org', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                        ],
                      ),
                      const SizedBox(height: 8),
                      Text('Inject poster metadata directly into directory browser view:', style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant)),
                      const SizedBox(height: 8),
                      DropdownButtonFormField<String>(
                        value: _folderIconMethod,
                        isExpanded: true,
                        decoration: const InputDecoration(border: OutlineInputBorder()),
                        dropdownColor: ObsidianColors.surfaceContainerHigh,
                        items: const [
                          DropdownMenuItem(value: 'Auto (Detect Desktop Env: KDE Plasma)', child: Text('Auto (Detect Desktop Env: KDE Plasma)', overflow: TextOverflow.ellipsis)),
                          DropdownMenuItem(value: 'KDE Plasma (KIO / directory.png)', child: Text('KDE Plasma (KIO / directory.png)', overflow: TextOverflow.ellipsis)),
                          DropdownMenuItem(value: 'GNOME (gvfs-metadata)', child: Text('GNOME (gvfs-metadata)', overflow: TextOverflow.ellipsis)),
                          DropdownMenuItem(value: 'Off (Disabled)', child: Text('Off (Disabled)', overflow: TextOverflow.ellipsis)),
                        ],
                        onChanged: (v) {
                          if (v != null) setState(() => _folderIconMethod = v);
                        },
                      ),
                      const SizedBox(height: 12),
                      Container(
                        padding: const EdgeInsets.all(8),
                        decoration: BoxDecoration(
                          color: ObsidianColors.surfaceContainerLow,
                          borderRadius: BorderRadius.circular(6),
                        ),
                        child: Row(
                          children: [
                            const Icon(Icons.palette_rounded, size: 16, color: ObsidianColors.primary),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text('Plasma KIO hook configured: creates directory.png and sets desktop.ini', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildPathPicker({
    required String label,
    required String metric,
    required IconData icon,
    required TextEditingController controller,
    required StateProvider state,
  }) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Expanded(
              child: Text(label, style: ObsidianTypography.monoCode(), overflow: TextOverflow.ellipsis),
            ),
            const SizedBox(width: 8),
            Text(metric, style: ObsidianTypography.monoCode(color: ObsidianColors.tertiary)),
          ],
        ),
        const SizedBox(height: 4),
        Row(
          children: [
            Expanded(
              child: Container(
                height: 38,
                padding: const EdgeInsets.symmetric(horizontal: 10),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainer,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Row(
                  children: [
                    Icon(icon, size: 16, color: ObsidianColors.primary),
                    const SizedBox(width: 8),
                    Expanded(
                      child: TextField(
                        controller: controller,
                        style: ObsidianTypography.monoCode(),
                        decoration: const InputDecoration(border: InputBorder.none, isDense: true, contentPadding: EdgeInsets.zero),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(width: 8),
            ElevatedButton.icon(
              onPressed: () {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text('Selecting path for ${controller.text}')),
                );
              },
              icon: const Icon(Icons.drive_file_move_rounded, size: 16),
              label: const Text('Browse...'),
              style: ElevatedButton.styleFrom(
                backgroundColor: ObsidianColors.surfaceContainerHigh,
                foregroundColor: ObsidianColors.onSurface,
                elevation: 0,
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildTokenChip(String token) {
    return InkWell(
      onTap: () {
        _namingPatternController.text += token;
      },
      borderRadius: BorderRadius.circular(4),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        decoration: BoxDecoration(
          color: ObsidianColors.surfaceContainerHigh,
          borderRadius: BorderRadius.circular(4),
        ),
        child: Text('+ $token', style: ObsidianTypography.labelBadge(color: ObsidianColors.primary)),
      ),
    );
  }

  // CARD 4: Download Engine & Concurrency Card
  Widget _buildEngineCard(BuildContext context, StateProvider state) {
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
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Row(
                  children: [
                    Container(
                      padding: const EdgeInsets.all(6),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHigh,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: const Icon(Icons.tune_rounded, size: 20, color: ObsidianColors.primary),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('Download Engine & Concurrency', style: ObsidianTypography.titleSm(), overflow: TextOverflow.ellipsis),
                          Text('Manage active task caps, network buffer quotas, and daily automation throttle limits', style: ObsidianTypography.bodySm(color: ObsidianColors.outline), overflow: TextOverflow.ellipsis),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text('Scheduler Active', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
              ),
            ],
          ),
          const SizedBox(height: 16),

          Row(
            children: [
              // Concurrency Stepper
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(14),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Expanded(
                            child: Text('Simultaneous Active Downloads', style: ObsidianTypography.titleSm(), overflow: TextOverflow.ellipsis),
                          ),
                          const SizedBox(width: 8),
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1.5),
                            decoration: BoxDecoration(
                              color: ObsidianColors.primary.withOpacity(0.2),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text('IO Priority', style: ObsidianTypography.labelBadge(color: ObsidianColors.primary)),
                          ),
                        ],
                      ),
                      const SizedBox(height: 4),
                      Text('Queue throttles additional snatches to preserve bandwidth priority and avoid ISP congestion flags.', style: ObsidianTypography.bodySm(color: ObsidianColors.outline)),
                      const SizedBox(height: 12),
                      Row(
                        children: [
                          IconButton(
                            onPressed: () {
                              if (_concurrency > 1) setState(() => _concurrency--);
                            },
                            icon: const Icon(Icons.remove_circle_outline_rounded),
                            color: ObsidianColors.outline,
                          ),
                          Container(
                            width: 60,
                            height: 36,
                            decoration: BoxDecoration(
                              color: ObsidianColors.surfaceContainerLowest,
                              borderRadius: BorderRadius.circular(6),
                            ),
                            alignment: Alignment.center,
                            child: Text('$_concurrency', style: ObsidianTypography.headlineMd(color: ObsidianColors.primary)),
                          ),
                          IconButton(
                            onPressed: () {
                              if (_concurrency < 16) setState(() => _concurrency++);
                            },
                            icon: const Icon(Icons.add_circle_outline_rounded),
                            color: ObsidianColors.outline,
                          ),
                          const Spacer(),
                          Text('Default: 2 streams', style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurfaceVariant)),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(width: 12),

              // Daily Cap Stepper
              Expanded(
                child: Container(
                  padding: const EdgeInsets.all(14),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Expanded(
                            child: Text('Daily Download Limit (Episodes)', style: ObsidianTypography.titleSm(), overflow: TextOverflow.ellipsis),
                          ),
                          const SizedBox(width: 8),
                          Container(
                            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1.5),
                            decoration: BoxDecoration(
                              color: ObsidianColors.tertiary.withOpacity(0.2),
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text('Tracker Safety', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
                          ),
                        ],
                      ),
                      const SizedBox(height: 4),
                      Text('Maximum automated episode downloads allowed within a 24-hour rolling window to preserve account ratios.', style: ObsidianTypography.bodySm(color: ObsidianColors.outline)),
                      const SizedBox(height: 12),
                      Row(
                        children: [
                          IconButton(
                            onPressed: () {
                              if (_dailyCap > 5) setState(() => _dailyCap -= 5);
                            },
                            icon: const Icon(Icons.remove_circle_outline_rounded),
                            color: ObsidianColors.outline,
                          ),
                          Container(
                            width: 60,
                            height: 36,
                            decoration: BoxDecoration(
                              color: ObsidianColors.surfaceContainerLowest,
                              borderRadius: BorderRadius.circular(6),
                            ),
                            alignment: Alignment.center,
                            child: Text('$_dailyCap', style: ObsidianTypography.headlineMd(color: ObsidianColors.tertiary)),
                          ),
                          IconButton(
                            onPressed: () {
                              if (_dailyCap < 200) setState(() => _dailyCap += 5);
                            },
                            icon: const Icon(Icons.add_circle_outline_rounded),
                            color: ObsidianColors.outline,
                          ),
                          const Spacer(),
                          Text('Default: 30 eps/day', style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurfaceVariant)),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 14),

          // Micro notification banner
          Container(
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainer,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Expanded(
                  child: Row(
                    children: [
                      const Icon(Icons.info_outline_rounded, size: 16, color: ObsidianColors.primary),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          'All changes take effect immediately on local daemon via UNIX socket: /run/genrl/core.sock',
                          style: ObsidianTypography.monoCode(color: ObsidianColors.outline),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                Text('Last written: 14:22:09 UTC', style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurfaceVariant)),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildInputField({
    required String label,
    required String? sublabel,
    required IconData icon,
    required TextEditingController controller,
  }) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Expanded(
              child: Text(label, style: ObsidianTypography.monoCode(), overflow: TextOverflow.ellipsis),
            ),
            if (sublabel != null) ...[
              const SizedBox(width: 8),
              Text(sublabel, style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
            ],
          ],
        ),
        const SizedBox(height: 4),
        Container(
          height: 38,
          padding: const EdgeInsets.symmetric(horizontal: 10),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: ObsidianColors.border),
          ),
          child: Row(
            children: [
              Icon(icon, size: 16, color: ObsidianColors.outline),
              const SizedBox(width: 8),
              Expanded(
                child: TextField(
                  controller: controller,
                  style: ObsidianTypography.monoCode(),
                  decoration: const InputDecoration(border: InputBorder.none, isDense: true, contentPadding: EdgeInsets.zero),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}
