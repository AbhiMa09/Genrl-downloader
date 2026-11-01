import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class MalConnectionControls extends StatefulWidget {
  final StateProvider state;
  const MalConnectionControls({super.key, required this.state});

  @override
  State<MalConnectionControls> createState() => _MalConnectionControlsState();
}

class _MalConnectionControlsState extends State<MalConnectionControls> {
  bool _opening = false;

  Future<void> _connect() async {
    if (_opening || widget.state.isConnectingMal) return;
    setState(() => _opening = true);
    try {
      await widget.state.refreshSettings();
      if (!mounted) return;
      final input = await showDialog<_MalCredentials>(
        context: context,
        builder: (_) => _MalCredentialsDialog(
          clientId:
              widget.state.settings['mal_client_id'] as String? ??
              widget.state.malClientId ??
              '',
          keyringAvailable: widget.state.keyringAvailable,
        ),
      );
      if (input != null) {
        await widget.state.connectMal(
          clientId: input.clientId.isEmpty ? null : input.clientId,
          clientSecret: input.clientSecret.isEmpty ? null : input.clientSecret,
          sessionOnly: input.sessionOnly,
        );
      }
    } finally {
      if (mounted) setState(() => _opening = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: widget.state,
      builder: (context, _) {
        final busy = _opening || widget.state.isConnectingMal;
        return Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            OutlinedButton.icon(
              key: const Key('mal_connect_browser'),
              onPressed: busy ? null : _connect,
              icon: const Icon(Icons.key_rounded, size: 16),
              label: Text(
                busy
                    ? 'Waiting for authorization…'
                    : widget.state.isMalAuthenticated
                    ? 'Reconnect'
                    : 'Connect via Browser (PKCE)',
              ),
              style: OutlinedButton.styleFrom(
                side: const BorderSide(color: ObsidianColors.border),
                foregroundColor: ObsidianColors.onSurface,
              ),
            ),
            if (widget.state.malError != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: SizedBox(
                  width: 320,
                  child: Text(
                    widget.state.malError!,
                    style: ObsidianTypography.bodySm(
                      color: ObsidianColors.error,
                    ),
                  ),
                ),
              ),
          ],
        );
      },
    );
  }
}

// Values live only in this UI request and the existing keyring/child environment.
// Never add a toString implementation that exposes the secret.
class _MalCredentials {
  final String clientId;
  final String clientSecret;
  final bool sessionOnly;
  const _MalCredentials(
    this.clientId,
    this.clientSecret, {
    this.sessionOnly = false,
  });
}

class _MalCredentialsDialog extends StatefulWidget {
  final String clientId;
  final bool keyringAvailable;
  const _MalCredentialsDialog({
    required this.clientId,
    this.keyringAvailable = true,
  });

  @override
  State<_MalCredentialsDialog> createState() => _MalCredentialsDialogState();
}

class _MalCredentialsDialogState extends State<_MalCredentialsDialog> {
  late final _clientId = TextEditingController(text: widget.clientId);
  final _secret = TextEditingController();
  final _form = GlobalKey<FormState>();
  bool _useCustomClient = false;
  late bool _sessionOnly = !widget.keyringAvailable;

  @override
  void dispose() {
    _clientId.dispose();
    _secret.clear();
    _secret.dispose();
    super.dispose();
  }

  void _authorize() {
    if (_useCustomClient && !_form.currentState!.validate()) return;
    final input = _MalCredentials(
      _useCustomClient ? _clientId.text.trim() : '',
      _useCustomClient ? _secret.text.trim() : '',
      sessionOnly: _sessionOnly,
    );
    _secret.clear();
    Navigator.of(context).pop(input);
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Connect MyAnimeList'),
      content: SizedBox(
        width: 420,
        child: SingleChildScrollView(
          child: Form(
            key: _form,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Authorize Genrl with your MyAnimeList account to synchronize your watching library and status using official OAuth2 PKCE.',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurfaceVariant,
                  ),
                ),
                if (!widget.keyringAvailable) ...[
                  const SizedBox(height: 12),
                  Container(
                    padding: const EdgeInsets.all(10),
                    decoration: BoxDecoration(
                      color: ObsidianColors.warning.withOpacity(0.12),
                      borderRadius: BorderRadius.circular(6),
                      border: Border.all(
                        color: ObsidianColors.warning.withOpacity(0.3),
                      ),
                    ),
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        const Icon(
                          Icons.info_outline_rounded,
                          size: 16,
                          color: ObsidianColors.warning,
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            'No desktop Secret Service / keyring detected (e.g. headless WSL). Credentials will be kept in memory for this session only.',
                            style: ObsidianTypography.labelBadge(
                              color: ObsidianColors.onSurface,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
                const SizedBox(height: 12),
                CheckboxListTile(
                  key: const Key('mal_session_only'),
                  value: _sessionOnly,
                  onChanged: (val) =>
                      setState(() => _sessionOnly = val ?? false),
                  title: const Text('Session only (memory only, no keyring)'),
                  subtitle: const Text(
                    'Refresh token is discarded when Genrl exits',
                  ),
                  dense: true,
                  contentPadding: EdgeInsets.zero,
                  controlAffinity: ListTileControlAffinity.leading,
                ),
                const SizedBox(height: 8),
                InkWell(
                  key: const Key('mal_advanced_toggle'),
                  onTap: () =>
                      setState(() => _useCustomClient = !_useCustomClient),
                  borderRadius: BorderRadius.circular(4),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(vertical: 4),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          _useCustomClient
                              ? Icons.arrow_drop_down
                              : Icons.arrow_right,
                          size: 20,
                          color: ObsidianColors.primary,
                        ),
                        Text(
                          'Advanced: use my own client ID',
                          style: ObsidianTypography.bodySm(
                            color: ObsidianColors.primary,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
                if (_useCustomClient) ...[
                  const SizedBox(height: 12),
                  TextFormField(
                    key: const Key('mal_client_id'),
                    controller: _clientId,
                    decoration: const InputDecoration(labelText: 'Client ID'),
                    validator: (value) => value == null || value.trim().isEmpty
                        ? 'Enter your MAL application Client ID.'
                        : null,
                  ),
                  const SizedBox(height: 14),
                  TextFormField(
                    key: const Key('mal_client_secret'),
                    controller: _secret,
                    obscureText: true,
                    enableSuggestions: false,
                    autocorrect: false,
                    decoration: const InputDecoration(
                      labelText: 'Client Secret (optional)',
                      helperText: 'Only if your custom MAL app requires one.',
                      helperMaxLines: 2,
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('mal_authorize'),
          onPressed: _authorize,
          child: const Text('Authorize in browser'),
        ),
      ],
    );
  }
}
