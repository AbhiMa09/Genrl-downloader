import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

/// Screen 3 of Guided First Run: Connect MyAnimeList (OAuth & Guest Mode).
class GuidedMalScreen extends StatefulWidget {
  final TextEditingController userController;
  final TextEditingController clientIdController;
  final TextEditingController? clientSecretController;
  final StateProvider state;

  const GuidedMalScreen({
    super.key,
    required this.userController,
    required this.clientIdController,
    this.clientSecretController,
    required this.state,
  });

  @override
  State<GuidedMalScreen> createState() => _GuidedMalScreenState();
}

class _GuidedMalScreenState extends State<GuidedMalScreen> {
  bool _isConnecting = false;
  bool _isGuestModeExpanded = false;
  bool _isCredentialsExpanded = false;
  String? _connectError;

  Future<void> _startOAuth() async {
    setState(() {
      _isConnecting = true;
      _connectError = null;
    });

    final clientId = widget.clientIdController.text.trim();
    final clientSecret = widget.clientSecretController?.text.trim();
    await widget.state.connectMal(
      clientId: clientId.isNotEmpty ? clientId : null,
      clientSecret: (clientSecret != null && clientSecret.isNotEmpty)
          ? clientSecret
          : null,
    );

    if (!mounted) return;

    if (!widget.state.isMalConnected) {
      setState(() {
        _connectError = widget.state.malError ?? widget.state.coreError ?? 'Could not initiate MyAnimeList authorization. Check network connection or verify Client ID.';
        _isConnecting = false;
      });
    } else {
      setState(() => _isConnecting = false);
    }
  }

  Future<void> _checkStatus() async {
    setState(() => _isConnecting = true);
    await widget.state.refreshStatus();
    if (mounted) setState(() => _isConnecting = false);
  }

  @override
  Widget build(BuildContext context) {
    final isConnected = widget.state.isMalConnected;

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
                    'Connect MyAnimeList',
                    style: ObsidianTypography.headlineMd(),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    'Sync your Watching, Plan to Watch, and Completed lists automatically.',
                    style: ObsidianTypography.bodySm(
                      color: ObsidianColors.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
            OutlinedButton.icon(
              onPressed: _isConnecting ? null : _checkStatus,
              icon: _isConnecting
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

        // Status Card
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
                    : Icons.account_circle_outlined,
                color: isConnected
                    ? ObsidianColors.tertiary
                    : ObsidianColors.primary,
                size: 22,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      isConnected
                          ? 'Connected to MyAnimeList as ${widget.state.malUsername ?? "user"}'
                          : 'Not connected yet',
                      style: ObsidianTypography.bodyMd(
                        color: isConnected
                            ? ObsidianColors.tertiary
                            : ObsidianColors.onSurface,
                      ).copyWith(fontWeight: FontWeight.w600),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      isConnected
                          ? 'Full OAuth sync active. Watches and episode updates will synchronize.'
                          : 'Click the button below to authorize Genrl via your web browser.',
                      style: ObsidianTypography.bodySm(
                        color: ObsidianColors.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),

        if (_connectError != null) ...[
          const SizedBox(height: 12),
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: ObsidianColors.error.withOpacity(0.1),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: ObsidianColors.error.withOpacity(0.5)),
            ),
            child: Row(
              children: [
                const Icon(
                  Icons.error_outline_rounded,
                  color: ObsidianColors.error,
                  size: 18,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    _connectError!,
                    style: ObsidianTypography.bodySm(
                      color: ObsidianColors.error,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],

        const SizedBox(height: 24),

        // Main OAuth Action Button
        Container(
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainerLow,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(color: ObsidianColors.border),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'OAuth Authorization (Recommended)',
                style: ObsidianTypography.titleSm(
                  color: ObsidianColors.onSurface,
                ),
              ),
              const SizedBox(height: 6),
              Text(
                'What will happen:\n1. Your web browser will open to the official MyAnimeList login page.\n2. Sign in to your MAL account and click "Allow".\n3. Come back to this window — Genrl will automatically save your session token securely.',
                style: ObsidianTypography.bodySm(
                  color: ObsidianColors.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 16),
              ElevatedButton.icon(
                onPressed: _isConnecting ? null : _startOAuth,
                icon: const Icon(Icons.open_in_browser_rounded, size: 18),
                label: const Text('Connect MyAnimeList'),
                style: ElevatedButton.styleFrom(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 20,
                    vertical: 12,
                  ),
                ),
              ),
            ],
          ),
        ),

        const SizedBox(height: 16),

        // Custom credentials expandable section
        if (widget.clientSecretController != null) ...[
          Material(
            color: ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(10),
            child: Container(
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(10),
                border: Border.all(color: ObsidianColors.border),
              ),
              child: ExpansionTile(
                initiallyExpanded: _isCredentialsExpanded,
                onExpansionChanged: (val) =>
                    setState(() => _isCredentialsExpanded = val),
                title: Text(
                  'Developer / Custom App Credentials',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurface,
                  ).copyWith(fontWeight: FontWeight.w600),
                ),
                subtitle: Text(
                  'Required if your MAL app was created with App Type: "web".',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.outline,
                  ),
                ),
                childrenPadding: const EdgeInsets.all(16),
                children: [
                  TextField(
                    controller: widget.clientIdController,
                    decoration: const InputDecoration(
                      labelText: 'Custom Client ID',
                      hintText: 'Leave empty for default app client ID',
                      prefixIcon: Icon(Icons.key_rounded),
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: widget.clientSecretController,
                    obscureText: true,
                    decoration: const InputDecoration(
                      labelText: 'Client Secret (Required for App Type "web")',
                      hintText: 'Paste secret from MAL developer portal',
                      prefixIcon: Icon(Icons.password_rounded),
                    ),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),
        ],

        // Guest mode expandable section
        Material(
          color: ObsidianColors.surfaceContainer,
          borderRadius: BorderRadius.circular(10),
          child: Container(
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: ObsidianColors.border),
            ),
            child: ExpansionTile(
              initiallyExpanded: _isGuestModeExpanded,
              onExpansionChanged: (val) =>
                  setState(() => _isGuestModeExpanded = val),
              title: Text(
                'Or use Guest Mode (Public username, read-only)',
                style: ObsidianTypography.bodySm(
                  color: ObsidianColors.onSurface,
                ).copyWith(fontWeight: FontWeight.w600),
              ),
              subtitle: Text(
                'No login required. Uses public Jikan API (rate limits apply).',
                style: ObsidianTypography.labelBadge(
                  color: ObsidianColors.outline,
                ),
              ),
              childrenPadding: const EdgeInsets.all(16),
              children: [
                TextField(
                  controller: widget.userController,
                  decoration: const InputDecoration(
                    labelText: 'Public MAL Username',
                    hintText: 'e.g. your_username',
                    prefixIcon: Icon(Icons.person_outline_rounded),
                  ),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: widget.clientIdController,
                  decoration: const InputDecoration(
                    labelText: 'Custom Client ID (Optional Developer Override)',
                    hintText: 'Leave empty for default public client ID',
                    prefixIcon: Icon(Icons.key_rounded),
                  ),
                ),
              ],
            ),
          ),
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
                  'Why do we ask this? Genrl reads your anime list so you never have to manually search for shows or calculate episode numbers.',
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
