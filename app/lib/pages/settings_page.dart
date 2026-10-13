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
  final _qbitUrlController = TextEditingController(text: 'http://localhost:8080');
  final _qbitUserController = TextEditingController(text: 'admin');
  final _malClientIdController = TextEditingController(text: 'c96e52a6f4b5ea18bc13f176ea27f0ae');
  final _malPortController = TextEditingController(text: '21445');
  final _publicUserController = TextEditingController(text: '');
  final _downloadFolderController = TextEditingController(text: '/home/genrl/Downloads');
  final _libraryFolderController = TextEditingController(text: '/mnt/anime');
  final _diskFloorController = TextEditingController(text: '15');
  final _scoreThresholdController = TextEditingController(text: '75.0');
  final _dailyCapController = TextEditingController(text: '30');

  bool _autostartQbit = false;
  bool _removeWhenFinished = true;
  String _folderIconMethod = 'Auto-detect (GNOME/KDE)';

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final state = context.read<StateProvider>();
      state.refreshMalStatus().then((_) {
        if (mounted) {
          if (state.malClientId != null && state.malClientId!.isNotEmpty && _malClientIdController.text.isEmpty) {
            _malClientIdController.text = state.malClientId!;
          }
          if (state.isPublicProfile && state.malUsername != null && state.malUsername!.isNotEmpty && _publicUserController.text.isEmpty) {
            _publicUserController.text = state.malUsername!;
          }
        }
      });
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
    _qbitUserController.dispose();
    _malClientIdController.dispose();
    _malPortController.dispose();
    _publicUserController.dispose();
    _downloadFolderController.dispose();
    _libraryFolderController.dispose();
    _diskFloorController.dispose();
    _scoreThresholdController.dispose();
    _dailyCapController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Text('Application Settings', style: ObsidianTypography.headlineLg()),
              const Spacer(),
              OutlinedButton.icon(
                onPressed: () {},
                icon: const Icon(Icons.file_upload_outlined, size: 16),
                label: const Text('Export JSON'),
                style: OutlinedButton.styleFrom(
                  side: const BorderSide(color: ObsidianColors.border),
                  foregroundColor: ObsidianColors.onSurface,
                ),
              ),
              const SizedBox(width: 8),
              OutlinedButton.icon(
                onPressed: () {},
                icon: const Icon(Icons.file_download_outlined, size: 16),
                label: const Text('Import JSON'),
                style: OutlinedButton.styleFrom(
                  side: const BorderSide(color: ObsidianColors.border),
                  foregroundColor: ObsidianColors.onSurface,
                ),
              ),
            ],
          ),
          const SizedBox(height: 20),

          // 1. Profile Card at Top
          _buildProfileCard(state),
          const SizedBox(height: 20),

          // 2. MAL Connection & OAuth Section
          _buildMalAccountSection(state),
          const SizedBox(height: 20),

          // 3. qBittorrent Configuration Card
          _SettingsSection(
            title: 'QBITTORRENT WEBUI CLIENT',
            icon: Icons.download_rounded,
            children: [
              _TextFieldRow(
                label: 'WebUI Endpoint URL',
                controller: _qbitUrlController,
                hintText: 'http://127.0.0.1:8080',
              ),
              const SizedBox(height: 12),
              _TextFieldRow(
                label: 'WebUI Username',
                controller: _qbitUserController,
                hintText: 'admin',
              ),
              const SizedBox(height: 12),
              _SwitchRow(
                label: 'Start qbittorrent-nox if not running',
                value: _autostartQbit,
                onChanged: (v) => setState(() => _autostartQbit = v),
              ),
              const SizedBox(height: 12),
              _SwitchRow(
                label: 'Remove torrent from client when organized into library',
                value: _removeWhenFinished,
                onChanged: (v) => setState(() => _removeWhenFinished = v),
              ),
              const SizedBox(height: 12),
              ElevatedButton.icon(
                onPressed: () {},
                icon: const Icon(Icons.check_circle_outline, size: 16),
                label: const Text('Test qBittorrent Connection'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainerHigh,
                  foregroundColor: ObsidianColors.primary,
                  elevation: 0,
                  side: const BorderSide(color: ObsidianColors.border),
                ),
              ),
            ],
          ),
          const SizedBox(height: 20),

          // 4. Folders & Storage
          _buildFoldersStorageSection(state),
          const SizedBox(height: 20),

          // 5. Automation & Guardrails
          _SettingsSection(
            title: 'AUTOMATION & RATE LIMITS',
            icon: Icons.tune_rounded,
            children: [
              _TextFieldRow(
                label: 'Candidate Score Threshold (0..100)',
                controller: _scoreThresholdController,
                hintText: '75.0',
              ),
              const SizedBox(height: 12),
              _TextFieldRow(
                label: 'Daily Torrents Add Cap',
                controller: _dailyCapController,
                hintText: '30',
              ),
            ],
          ),
          const SizedBox(height: 20),

          // 6. Desktop Folder Icons
          _SettingsSection(
            title: 'LINUX DESKTOP ICONS',
            icon: Icons.palette_outlined,
            children: [
              Row(
                children: [
                  Text('Icon Method:', style: ObsidianTypography.bodySm()),
                  const SizedBox(width: 16),
                  DropdownButton<String>(
                    value: _folderIconMethod,
                    dropdownColor: ObsidianColors.surfaceContainerHigh,
                    underline: const SizedBox(),
                    items: const [
                      DropdownMenuItem(
                          value: 'Auto-detect (GNOME/KDE)',
                          child: Text('Auto-detect (GNOME/KDE)')),
                      DropdownMenuItem(
                          value: 'KDE Plasma (.directory)',
                          child: Text('KDE Plasma (.directory)')),
                      DropdownMenuItem(
                          value: 'GNOME (gio custom-icon)',
                          child: Text('GNOME (gio custom-icon)')),
                      DropdownMenuItem(
                          value: 'Disabled', child: Text('Disabled')),
                    ],
                    onChanged: (val) {
                      if (val != null) setState(() => _folderIconMethod = val);
                    },
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildProfileCard(StateProvider state) {
    final username = state.malUsername ?? (state.isMalConnected ? 'MAL User' : 'Not Connected');
    final statusBadge = _getStatusBadge(state.malStatus);

    return Container(
      padding: const EdgeInsets.all(20),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Row(
        children: [
          CircleAvatar(
            radius: 30,
            backgroundColor: ObsidianColors.primary.withOpacity(0.15),
            child: Icon(
              state.isMalConnected ? Icons.person_rounded : Icons.person_outline_rounded,
              size: 32,
              color: ObsidianColors.primary,
            ),
          ),
          const SizedBox(width: 20),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text(username, style: ObsidianTypography.headlineMd()),
                    const SizedBox(width: 10),
                    statusBadge,
                  ],
                ),
                const SizedBox(height: 6),
                Text(
                  'Last Sync: ${state.status.formattedLastSync} • Source: ${state.malSource?.toUpperCase() ?? "NONE"}',
                  style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                ),
                const SizedBox(height: 12),
                Wrap(
                  spacing: 16,
                  runSpacing: 8,
                  children: [
                    _CountItem(label: 'Watching', count: state.watchingCount, color: ObsidianColors.primary),
                    _CountItem(label: 'Plan to Watch', count: state.planToWatchCount, color: ObsidianColors.onSurfaceVariant),
                    _CountItem(label: 'Completed', count: state.completedCount, color: ObsidianColors.success),
                    _CountItem(label: 'On Hold', count: state.onHoldCount, color: ObsidianColors.tertiary),
                    _CountItem(label: 'Dropped', count: state.droppedCount, color: ObsidianColors.error),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _getStatusBadge(String status) {
    Color bg;
    Color fg;
    String text;

    switch (status) {
      case 'connected':
        bg = ObsidianColors.success.withOpacity(0.15);
        fg = ObsidianColors.success;
        text = 'Connected (OAuth)';
        break;
      case 'public_profile':
        bg = ObsidianColors.tertiary.withOpacity(0.15);
        fg = ObsidianColors.tertiary;
        text = 'Public Profile (Jikan)';
        break;
      case 'token_expired':
        bg = ObsidianColors.error.withOpacity(0.15);
        fg = ObsidianColors.error;
        text = 'Token Expired';
        break;
      default:
        bg = ObsidianColors.outline.withOpacity(0.15);
        fg = ObsidianColors.outline;
        text = 'Not Connected';
        break;
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(text, style: ObsidianTypography.labelBadge(color: fg)),
    );
  }

  Widget _buildMalAccountSection(StateProvider state) {
    final port = int.tryParse(_malPortController.text.trim()) ?? 21445;

    return _SettingsSection(
      title: 'MYANIMELIST ACCOUNT & OAUTH (PKCE)',
      icon: Icons.person_outline_rounded,
      children: [
        _TextFieldRow(
          label: 'MAL Client ID (from myanimelist.net/apiconfig)',
          controller: _malClientIdController,
          hintText: 'Enter your 32-character Client ID',
        ),
        const SizedBox(height: 12),
        _TextFieldRow(
          label: 'OAuth Redirect Port (default 21445)',
          controller: _malPortController,
          hintText: '21445',
        ),
        const SizedBox(height: 16),

        // Action Buttons Row: Connect, Disconnect, Sync Now
        Wrap(
          spacing: 12,
          runSpacing: 10,
          children: [
            ElevatedButton.icon(
              onPressed: () => state.connectMal(
                clientId: _malClientIdController.text.trim().isNotEmpty
                    ? _malClientIdController.text.trim()
                    : null,
                port: port,
              ),
              icon: const Icon(Icons.login_rounded, size: 16),
              label: const Text('Connect via Browser (PKCE)'),
              style: ElevatedButton.styleFrom(
                backgroundColor: ObsidianColors.primaryContainer,
                foregroundColor: ObsidianColors.onPrimaryContainer,
                elevation: 0,
              ),
            ),
            if (state.isMalConnected || state.isPublicProfile) ...[
              OutlinedButton.icon(
                onPressed: () => state.disconnectMal(),
                icon: const Icon(Icons.logout_rounded, size: 16),
                label: const Text('Disconnect'),
                style: OutlinedButton.styleFrom(
                  side: const BorderSide(color: ObsidianColors.error),
                  foregroundColor: ObsidianColors.error,
                ),
              ),
              ElevatedButton.icon(
                onPressed: state.isSyncing ? null : () => state.forceSyncMal(),
                icon: state.isSyncing
                    ? const SizedBox(width: 14, height: 14, child: CircularProgressIndicator(strokeWidth: 2))
                    : const Icon(Icons.sync_rounded, size: 16),
                label: Text(state.isSyncing ? 'Syncing...' : 'Sync Now'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.surfaceContainerHigh,
                  foregroundColor: ObsidianColors.onSurface,
                  elevation: 0,
                  side: const BorderSide(color: ObsidianColors.border),
                ),
              ),
            ],
          ],
        ),

        const SizedBox(height: 14),
        Text(
          'OAuth2 PKCE (method: "plain") only. Tokens are encrypted and stored in the system keyring.',
          style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
        ),

        // Error Banner & Checklist if connect fails
        if (state.malError != null && state.malError!.isNotEmpty) ...[
          const SizedBox(height: 16),
          Container(
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: ObsidianColors.error.withOpacity(0.12),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: ObsidianColors.error.withOpacity(0.4)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    const Icon(Icons.error_outline_rounded, size: 18, color: ObsidianColors.error),
                    const SizedBox(width: 8),
                    Text('MAL Connection Error', style: ObsidianTypography.titleSm(color: ObsidianColors.error)),
                  ],
                ),
                const SizedBox(height: 8),
                Text(
                  state.malError!,
                  style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
                ),
                const SizedBox(height: 10),
                Text(
                  'Checklist to verify on https://myanimelist.net/apiconfig:',
                  style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface).copyWith(fontWeight: FontWeight.w600),
                ),
                const SizedBox(height: 4),
                Text(
                  '1. Client ID matches exactly.\n'
                  '2. App Redirect URL is registered as: http://127.0.0.1:$port/callback (or http://localhost:$port/callback)\n'
                  '3. App Type is set to "Web" or "Other".',
                  style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                ),
              ],
            ),
          ),
        ],

        const SizedBox(height: 20),
        const Divider(color: ObsidianColors.border),
        const SizedBox(height: 16),

        // Fallback: Use Public Profile
        Text('FALLBACK: USE PUBLIC PROFILE', style: ObsidianTypography.titleSm()),
        const SizedBox(height: 6),
        Text(
          'No MAL API account required. Fetches your anime list via Jikan public profile endpoint (read-only).',
          style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(
              child: Container(
                height: 38,
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLowest,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: ObsidianColors.border),
                ),
                padding: const EdgeInsets.symmetric(horizontal: 12),
                alignment: Alignment.centerLeft,
                child: TextField(
                  controller: _publicUserController,
                  style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface),
                  decoration: InputDecoration(
                    hintText: 'Enter public MAL username (e.g. AnimeFan123)',
                    hintStyle: ObsidianTypography.monoCode(color: ObsidianColors.outline),
                    border: InputBorder.none,
                    isDense: true,
                    contentPadding: EdgeInsets.zero,
                  ),
                ),
              ),
            ),
            const SizedBox(width: 10),
            ElevatedButton(
              onPressed: () {
                final user = _publicUserController.text.trim();
                if (user.isNotEmpty) {
                  state.setPublicProfile(user);
                }
              },
              style: ElevatedButton.styleFrom(
                backgroundColor: ObsidianColors.surfaceContainerHigh,
                foregroundColor: ObsidianColors.primary,
                elevation: 0,
                side: const BorderSide(color: ObsidianColors.border),
              ),
              child: const Text('Save Public Username'),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildFoldersStorageSection(StateProvider state) {
    return _SettingsSection(
      title: 'FOLDERS & STORAGE LIMITS',
      icon: Icons.folder_open_rounded,
      children: [
        // Download Folder Row
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: _TextFieldRow(
                    label: 'Download Folder (Incoming staging)',
                    controller: _downloadFolderController,
                    hintText: '/home/user/Downloads/Genrl',
                  ),
                ),
                const SizedBox(width: 8),
                Padding(
                  padding: const EdgeInsets.only(top: 22),
                  child: ElevatedButton(
                    onPressed: () async {
                      final path = _downloadFolderController.text.trim();
                      if (path.isNotEmpty) {
                        final ok = await state.saveLocation('download_dir', path);
                        if (mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(
                              content: Text(ok ? 'Download folder saved' : 'Invalid download folder path'),
                              backgroundColor: ok ? ObsidianColors.success : ObsidianColors.error,
                            ),
                          );
                        }
                      }
                    },
                    style: ElevatedButton.styleFrom(
                      backgroundColor: ObsidianColors.surfaceContainerHigh,
                      foregroundColor: ObsidianColors.primary,
                      elevation: 0,
                      side: const BorderSide(color: ObsidianColors.border),
                    ),
                    child: const Text('Save'),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 6),
            _buildFolderValidationBadge(state.downloadCheck),
          ],
        ),
        const SizedBox(height: 16),

        // Library Folder Row
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: _TextFieldRow(
                    label: 'Library Folder (Final library destination)',
                    controller: _libraryFolderController,
                    hintText: '/home/user/Videos/Anime',
                  ),
                ),
                const SizedBox(width: 8),
                Padding(
                  padding: const EdgeInsets.only(top: 22),
                  child: ElevatedButton(
                    onPressed: () async {
                      final path = _libraryFolderController.text.trim();
                      if (path.isNotEmpty) {
                        final ok = await state.saveLocation('library_dir', path);
                        if (mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(
                              content: Text(ok ? 'Library folder saved' : 'Invalid library folder path'),
                              backgroundColor: ok ? ObsidianColors.success : ObsidianColors.error,
                            ),
                          );
                        }
                      }
                    },
                    style: ElevatedButton.styleFrom(
                      backgroundColor: ObsidianColors.surfaceContainerHigh,
                      foregroundColor: ObsidianColors.primary,
                      elevation: 0,
                      side: const BorderSide(color: ObsidianColors.border),
                    ),
                    child: const Text('Save'),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 6),
            _buildFolderValidationBadge(state.libraryCheck),
          ],
        ),
        const SizedBox(height: 16),

        // Filesystem Status Banner (Same FS hardlink vs Cross-device move)
        Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: state.sameFilesystem
                ? ObsidianColors.success.withOpacity(0.08)
                : ObsidianColors.tertiary.withOpacity(0.08),
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: state.sameFilesystem
                  ? ObsidianColors.success.withOpacity(0.3)
                  : ObsidianColors.tertiary.withOpacity(0.3),
            ),
          ),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(
                state.sameFilesystem ? Icons.link_rounded : Icons.drive_file_move_outline,
                size: 20,
                color: state.sameFilesystem ? ObsidianColors.success : ObsidianColors.tertiary,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      state.sameFilesystem
                          ? 'Same Filesystem (Hardlink Enabled)'
                          : 'Different Filesystems (Cross-Device Move)',
                      style: ObsidianTypography.titleSm(
                        color: state.sameFilesystem ? ObsidianColors.success : ObsidianColors.tertiary,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      state.filesystemNote.isNotEmpty
                          ? state.filesystemNote
                          : (state.sameFilesystem
                              ? 'Download folder and Library folder are on the same filesystem. Files are instantly hardlinked so qBittorrent continues seeding.'
                              : 'Folders are on different drives/mounts. Files will be copied across devices (seeding stops after move).'),
                      style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),

        // Disk Floor Row
        _TextFieldRow(
          label: 'Disk Floor (GB free before pausing queue)',
          controller: _diskFloorController,
          hintText: '15',
        ),
        const SizedBox(height: 16),

        // Explicit Migration Action
        Row(
          children: [
            OutlinedButton.icon(
              onPressed: () => _showMigrateDialog(context, state),
              icon: const Icon(Icons.drive_file_move_rounded, size: 16),
              label: const Text('Migrate Existing Library...'),
              style: OutlinedButton.styleFrom(
                side: const BorderSide(color: ObsidianColors.border),
                foregroundColor: ObsidianColors.onSurface,
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                'Changing location only affects new downloads. Use Migrate to move existing files.',
                style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildFolderValidationBadge(Map<String, dynamic>? check) {
    if (check == null) return const SizedBox.shrink();

    final exists = check['exists'] == true;
    final writable = check['writable'] == true;
    final freeGb = check['free_space_gb'] as num?;
    final error = check['error'] as String?;

    return Wrap(
      spacing: 8,
      runSpacing: 4,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
          decoration: BoxDecoration(
            color: (exists ? ObsidianColors.success : ObsidianColors.outline).withOpacity(0.12),
            borderRadius: BorderRadius.circular(4),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                exists ? Icons.check_circle_outline : Icons.info_outline,
                size: 12,
                color: exists ? ObsidianColors.success : ObsidianColors.outline,
              ),
              const SizedBox(width: 4),
              Text(
                exists ? 'Exists' : 'Will be created',
                style: ObsidianTypography.labelBadge(
                  color: exists ? ObsidianColors.success : ObsidianColors.outline,
                ),
              ),
            ],
          ),
        ),
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
          decoration: BoxDecoration(
            color: (writable ? ObsidianColors.success : ObsidianColors.error).withOpacity(0.12),
            borderRadius: BorderRadius.circular(4),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                writable ? Icons.edit_outlined : Icons.lock_outline,
                size: 12,
                color: writable ? ObsidianColors.success : ObsidianColors.error,
              ),
              const SizedBox(width: 4),
              Text(
                writable ? 'Writable' : 'Permission Denied',
                style: ObsidianTypography.labelBadge(
                  color: writable ? ObsidianColors.success : ObsidianColors.error,
                ),
              ),
            ],
          ),
        ),
        if (freeGb != null)
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
            decoration: BoxDecoration(
              color: ObsidianColors.primary.withOpacity(0.12),
              borderRadius: BorderRadius.circular(4),
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(Icons.storage_rounded, size: 12, color: ObsidianColors.primary),
                const SizedBox(width: 4),
                Text(
                  '${freeGb.toStringAsFixed(1)} GiB Free',
                  style: ObsidianTypography.labelBadge(color: ObsidianColors.primary),
                ),
              ],
            ),
          ),
        if (error != null && error.isNotEmpty)
          Text(
            error,
            style: ObsidianTypography.bodySm(color: ObsidianColors.error),
          ),
      ],
    );
  }

  void _showMigrateDialog(BuildContext context, StateProvider state) {
    final fromController = TextEditingController(text: state.libraryCheck?['path'] ?? _libraryFolderController.text);
    final toController = TextEditingController(text: _libraryFolderController.text);

    showDialog(
      context: context,
      builder: (ctx) {
        return StatefulBuilder(
          builder: (ctx, setDialogState) {
            return AlertDialog(
              backgroundColor: ObsidianColors.surfaceContainerLow,
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
              title: Row(
                children: [
                  const Icon(Icons.drive_file_move_rounded, color: ObsidianColors.primary),
                  const SizedBox(width: 10),
                  Text('Migrate Existing Library', style: ObsidianTypography.headlineMd()),
                ],
              ),
              content: SizedBox(
                width: 500,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'This moves existing anime folders to the new location and updates their paths in the database. Never moves files automatically.',
                      style: ObsidianTypography.bodyMd(color: ObsidianColors.onSurfaceVariant),
                    ),
                    const SizedBox(height: 16),
                    TextField(
                      controller: fromController,
                      style: ObsidianTypography.monoCode(),
                      decoration: const InputDecoration(
                        labelText: 'Source Folder (Current Library)',
                        border: OutlineInputBorder(),
                        isDense: true,
                      ),
                    ),
                    const SizedBox(height: 12),
                    TextField(
                      controller: toController,
                      style: ObsidianTypography.monoCode(),
                      decoration: const InputDecoration(
                        labelText: 'Target Folder (Destination Library)',
                        border: OutlineInputBorder(),
                        isDense: true,
                      ),
                    ),
                    if (state.isMigrating) ...[
                      const SizedBox(height: 16),
                      const LinearProgressIndicator(),
                      const SizedBox(height: 8),
                      Text('Moving files and updating database...', style: ObsidianTypography.bodySm()),
                    ],
                    if (state.migrationResult != null) ...[
                      const SizedBox(height: 16),
                      Container(
                        padding: const EdgeInsets.all(10),
                        decoration: BoxDecoration(
                          color: ObsidianColors.surfaceContainerLowest,
                          borderRadius: BorderRadius.circular(6),
                        ),
                        child: Text(
                          state.migrationResult!,
                          style: ObsidianTypography.monoCode(),
                        ),
                      ),
                    ],
                  ],
                ),
              ),
              actions: [
                TextButton(
                  onPressed: state.isMigrating ? null : () => Navigator.of(ctx).pop(),
                  child: const Text('Close'),
                ),
                ElevatedButton(
                  onPressed: state.isMigrating
                      ? null
                      : () async {
                          final from = fromController.text.trim();
                          final to = toController.text.trim();
                          if (from.isNotEmpty && to.isNotEmpty && from != to) {
                            setDialogState(() {});
                            await state.migrateLibrary(from, to);
                            setDialogState(() {});
                          }
                        },
                  style: ElevatedButton.styleFrom(
                    backgroundColor: ObsidianColors.primaryContainer,
                    foregroundColor: ObsidianColors.onPrimaryContainer,
                  ),
                  child: const Text('Confirm & Migrate'),
                ),
              ],
            );
          },
        );
      },
    );
  }
}

class _CountItem extends StatelessWidget {
  final String label;
  final int count;
  final Color color;

  const _CountItem({
    required this.label,
    required this.count,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: 8,
          height: 8,
          decoration: BoxDecoration(color: color, shape: BoxShape.circle),
        ),
        const SizedBox(width: 6),
        Text(
          '$label: ',
          style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
        ),
        Text(
          count.toString(),
          style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface).copyWith(fontWeight: FontWeight.w600),
        ),
      ],
    );
  }
}

class _SettingsSection extends StatelessWidget {
  final String title;
  final IconData icon;
  final List<Widget> children;

  const _SettingsSection({
    required this.title,
    required this.icon,
    required this.children,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(18),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border, width: 1),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(icon, size: 18, color: ObsidianColors.primary),
              const SizedBox(width: 8),
              Text(title, style: ObsidianTypography.titleSm()),
            ],
          ),
          const SizedBox(height: 16),
          ...children,
        ],
      ),
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

class _SwitchRow extends StatelessWidget {
  final String label;
  final bool value;
  final ValueChanged<bool> onChanged;

  const _SwitchRow({
    required this.label,
    required this.value,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(label, style: ObsidianTypography.bodySm()),
        Switch(
          value: value,
          onChanged: onChanged,
          activeColor: ObsidianColors.primary,
        ),
      ],
    );
  }
}
