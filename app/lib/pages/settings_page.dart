import 'package:flutter/material.dart';
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
  final _malClientIdController = TextEditingController(text: '');
  final _downloadFolderController = TextEditingController(text: '/home/genrl/Downloads');
  final _libraryFolderController = TextEditingController(text: '/mnt/anime');
  final _diskFloorController = TextEditingController(text: '15');
  final _scoreThresholdController = TextEditingController(text: '75.0');
  final _dailyCapController = TextEditingController(text: '30');

  bool _autostartQbit = false;
  bool _removeWhenFinished = true;
  String _folderIconMethod = 'Auto-detect (GNOME/KDE)';

  @override
  Widget build(BuildContext context) {
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
          const SizedBox(height: 24),

          // 1. qBittorrent Configuration Card
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

          // 2. MyAnimeList OAuth Card
          _SettingsSection(
            title: 'MYANIMELIST API & OAUTH (PKCE)',
            icon: Icons.person_outline_rounded,
            children: [
              _TextFieldRow(
                label: 'MAL Client ID (from myanimelist.net/apiconfig)',
                controller: _malClientIdController,
                hintText: 'Enter your 32-character Client ID',
              ),
              const SizedBox(height: 14),
              Row(
                children: [
                  ElevatedButton.icon(
                    onPressed: () {},
                    icon: const Icon(Icons.login_rounded, size: 16),
                    label: const Text('Connect via Browser (PKCE)'),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: ObsidianColors.primaryContainer,
                      foregroundColor: ObsidianColors.onPrimaryContainer,
                      elevation: 0,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Text(
                    'Tokens are encrypted and stored in system keyring.',
                    style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 20),

          // 3. Folders & Storage
          _SettingsSection(
            title: 'FOLDERS & STORAGE LIMITS',
            icon: Icons.folder_open_rounded,
            children: [
              _TextFieldRow(
                label: 'Download Folder (qBittorrent staging)',
                controller: _downloadFolderController,
                hintText: '/home/user/Downloads',
              ),
              const SizedBox(height: 12),
              _TextFieldRow(
                label: 'Library Folder (final destination)',
                controller: _libraryFolderController,
                hintText: '/mnt/anime',
              ),
              const SizedBox(height: 12),
              _TextFieldRow(
                label: 'Disk Floor (GB free before pausing queue)',
                controller: _diskFloorController,
                hintText: '15',
              ),
            ],
          ),
          const SizedBox(height: 20),

          // 4. Automation & Guardrails
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

          // 5. Desktop Folder Icons
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
