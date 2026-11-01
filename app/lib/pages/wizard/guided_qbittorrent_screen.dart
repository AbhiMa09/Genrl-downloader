import 'dart:io';

import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

/// Screen 2 of Guided First Run: qBittorrent Detection, Instructions, & Live Probe.
class GuidedQbittorrentScreen extends StatefulWidget {
  final TextEditingController urlController;
  final TextEditingController userController;
  final TextEditingController passwordController;
  final StateProvider state;

  const GuidedQbittorrentScreen({
    super.key,
    required this.urlController,
    required this.userController,
    required this.passwordController,
    required this.state,
  });

  @override
  State<GuidedQbittorrentScreen> createState() =>
      _GuidedQbittorrentScreenState();
}

class _GuidedQbittorrentScreenState extends State<GuidedQbittorrentScreen> {
  bool _isTesting = false;
  String? _probeFailureMessage;

  @override
  void initState() {
    super.initState();
    if (widget.urlController.text.isEmpty) {
      widget.urlController.text = 'http://127.0.0.1:8080';
    }
  }

  bool _isQbitInstalledOnWindows() {
    if (!Platform.isWindows) return true;
    final programFiles =
        Platform.environment['ProgramFiles'] ?? r'C:\Program Files';
    final programFilesX86 =
        Platform.environment['ProgramFiles(x86)'] ?? r'C:\Program Files (x86)';
    final localAppData = Platform.environment['LOCALAPPDATA'] ?? '';

    final candidatePaths = [
      '$programFiles\\qBittorrent\\qbittorrent.exe',
      '$programFilesX86\\qBittorrent\\qbittorrent.exe',
      if (localAppData.isNotEmpty)
        '$localAppData\\Programs\\qBittorrent\\qbittorrent.exe',
    ];

    return candidatePaths.any((p) => File(p).existsSync());
  }

  Future<void> _testConnection() async {
    setState(() {
      _isTesting = true;
      _probeFailureMessage = null;
    });

    await widget.state.saveQbitEndpoint(widget.urlController.text);
    await widget.state.refreshStatus();

    if (!mounted) return;

    if (!widget.state.status.qbitConnected) {
      final endpoint = widget.urlController.text.trim();
      final port = RegExp(r':(\d+)').firstMatch(endpoint)?.group(1) ?? '8080';

      if (!_isQbitInstalledOnWindows()) {
        _probeFailureMessage = 'qBittorrent was not detected on this system. Please click "Download qBittorrent" below to install the official release.';
      } else if (endpoint.contains('127.0.0.1') ||
          endpoint.contains('localhost')) {
        _probeFailureMessage =
            'Connection refused on port $port. qBittorrent may not be running, or Web UI is disabled.\nFix: Launch qBittorrent, open Tools -> Options -> Web UI, check "Web User Interface", set port to $port, and check "Bypass authentication for clients on localhost".';
      } else {
        _probeFailureMessage =
            'Cannot reach Web UI at $endpoint. Check network address, verify qBittorrent is running, and ensure firewall permits incoming connections.';
      }
    } else {
      _probeFailureMessage = null;
    }

    setState(() => _isTesting = false);
  }

  void _openDownloadSite() {
    const downloadUrl = 'https://www.qbittorrent.org/download';
    if (Platform.isWindows) {
      Process.run('explorer.exe', [downloadUrl]);
    } else if (Platform.isLinux) {
      Process.run('xdg-open', [downloadUrl]);
    } else if (Platform.isMacOS) {
      Process.run('open', [downloadUrl]);
    }
  }

  @override
  Widget build(BuildContext context) {
    final isConnected = widget.state.status.qbitConnected;
    final isInstalled = _isQbitInstalledOnWindows();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Connect qBittorrent',
                    style: ObsidianTypography.headlineMd(),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    'Genrl uses qBittorrent Web UI to add and monitor episode downloads.',
                    style: ObsidianTypography.bodySm(
                      color: ObsidianColors.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
            OutlinedButton.icon(
              onPressed: _isTesting ? null : _testConnection,
              icon: _isTesting
                  ? const SizedBox(
                      width: 14,
                      height: 14,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.refresh_rounded, size: 16),
              label: const Text('Check again'),
            ),
          ],
        ),
        const SizedBox(height: 20),

        // Connection status banner
        Container(
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(
            color: isConnected
                ? ObsidianColors.tertiary.withOpacity(0.12)
                : ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(10),
            border: Border.all(
              color: isConnected
                  ? ObsidianColors.tertiary
                  : ObsidianColors.border,
            ),
          ),
          child: Row(
            children: [
              Icon(
                isConnected
                    ? Icons.check_circle_rounded
                    : Icons.info_outline_rounded,
                color: isConnected
                    ? ObsidianColors.tertiary
                    : ObsidianColors.outline,
                size: 20,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  isConnected
                      ? 'qBittorrent ${widget.state.status.qbitVersion} connected on ${widget.urlController.text}!'
                      : 'Not connected. Follow the steps below to configure qBittorrent Web UI.',
                  style: ObsidianTypography.bodyMd(
                    color: isConnected
                        ? ObsidianColors.tertiary
                        : ObsidianColors.onSurface,
                  ),
                ),
              ),
              if (!isConnected)
                ElevatedButton(
                  onPressed: _isTesting ? null : _testConnection,
                  child: const Text('Test Connection'),
                ),
            ],
          ),
        ),

        if (_probeFailureMessage != null) ...[
          const SizedBox(height: 12),
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: ObsidianColors.error.withOpacity(0.1),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: ObsidianColors.error.withOpacity(0.5)),
            ),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Icon(
                  Icons.error_outline_rounded,
                  color: ObsidianColors.error,
                  size: 18,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    _probeFailureMessage!,
                    style: ObsidianTypography.bodySm(
                      color: ObsidianColors.error,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],

        if (!isInstalled && !isConnected) ...[
          const SizedBox(height: 16),
          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerHigh,
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: ObsidianColors.border),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'qBittorrent is not installed',
                  style: ObsidianTypography.titleSm(
                    color: ObsidianColors.onSurface,
                  ),
                ),
                const SizedBox(height: 6),
                Text(
                  'qBittorrent is the recommended torrent client for Genrl Downloader. Download and install it from the official website.',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 12),
                ElevatedButton.icon(
                  onPressed: _openDownloadSite,
                  icon: const Icon(Icons.open_in_browser_rounded, size: 16),
                  label: const Text('Download qBittorrent (Official Site)'),
                ),
              ],
            ),
          ),
        ],

        const SizedBox(height: 20),
        Text(
          'Numbered Setup Instructions:',
          style: ObsidianTypography.titleSm(color: ObsidianColors.onSurface),
        ),
        const SizedBox(height: 10),
        const _StepGuideItem(
          step: '1',
          text: 'Launch qBittorrent and click Tools -> Options (or press Alt + O).',
        ),
        const SizedBox(height: 8),
        const _StepGuideItem(
          step: '2',
          text: 'In the left sidebar, select Web UI and check "Web User Interface (Remote control)".',
        ),
        const SizedBox(height: 8),
        const _StepGuideItem(
          step: '3',
          text: 'Set IP Address to 127.0.0.1 and Port to 8080 (or 18080).',
        ),
        const SizedBox(height: 8),
        const _StepGuideItem(
          step: '4',
          text: 'Under Authentication, check "Bypass authentication for clients on localhost".',
        ),
        const SizedBox(height: 8),
        const _StepGuideItem(
          step: '5',
          text: 'Click Apply and OK, then click "Test Connection" or "Check again" above.',
        ),

        const SizedBox(height: 20),
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: widget.urlController,
                decoration: const InputDecoration(
                  labelText: 'WebUI Endpoint URL',
                  hintText: 'http://127.0.0.1:8080',
                  prefixIcon: Icon(Icons.link_rounded),
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: TextField(
                controller: widget.userController,
                decoration: const InputDecoration(
                  labelText: 'Username (Optional if bypassed)',
                  hintText: 'admin',
                  prefixIcon: Icon(Icons.person_outline_rounded),
                ),
              ),
            ),
          ],
        ),

        const SizedBox(height: 20),
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainerHighest.withOpacity(0.5),
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: ObsidianColors.border.withOpacity(0.5)),
          ),
          child: Row(
            children: [
              const Icon(
                Icons.info_outline_rounded,
                size: 16,
                color: ObsidianColors.primary,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  'Why do we ask this? Genrl communicates with qBittorrent strictly locally over HTTP to pause, resume, and file your torrents automatically.',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurfaceVariant,
                  ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _StepGuideItem extends StatelessWidget {
  final String step;
  final String text;

  const _StepGuideItem({required this.step, required this.text});

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          width: 22,
          height: 22,
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainerHigh,
            borderRadius: BorderRadius.circular(6),
            border: Border.all(color: ObsidianColors.border),
          ),
          alignment: Alignment.center,
          child: Text(
            step,
            style: ObsidianTypography.labelBadge(color: ObsidianColors.primary),
          ),
        ),
        const SizedBox(width: 10),
        Expanded(
          child: Text(
            text,
            style: ObsidianTypography.bodySm(color: ObsidianColors.onSurface),
          ),
        ),
      ],
    );
  }
}
