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
        ),
      );
      if (input != null) {
        await widget.state.connectMal(
          clientId: input.clientId,
          clientSecret: input.clientSecret.isEmpty ? null : input.clientSecret,
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
  const _MalCredentials(this.clientId, this.clientSecret);
}

class _MalCredentialsDialog extends StatefulWidget {
  final String clientId;
  const _MalCredentialsDialog({required this.clientId});

  @override
  State<_MalCredentialsDialog> createState() => _MalCredentialsDialogState();
}

class _MalCredentialsDialogState extends State<_MalCredentialsDialog> {
  late final _clientId = TextEditingController(text: widget.clientId);
  final _secret = TextEditingController();
  final _form = GlobalKey<FormState>();

  @override
  void dispose() {
    _clientId.dispose();
    _secret.clear();
    _secret.dispose();
    super.dispose();
  }

  void _authorize() {
    if (!_form.currentState!.validate()) return;
    final input = _MalCredentials(_clientId.text.trim(), _secret.text.trim());
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
              children: [
                TextFormField(
                  key: const Key('mal_client_id'),
                  controller: _clientId,
                  decoration: const InputDecoration(labelText: 'Client ID'),
                  validator: (value) => value == null || value.trim().isEmpty
                      ? 'Enter your MAL application Client ID.'
                      : null,
                ),
                const SizedBox(height: 16),
                TextFormField(
                  key: const Key('mal_client_secret'),
                  controller: _secret,
                  obscureText: true,
                  enableSuggestions: false,
                  autocorrect: false,
                  decoration: const InputDecoration(
                    labelText: 'Client Secret (optional)',
                    helperText: 'Only if your MAL app requires one. Stored in the local keyring.',
                    helperMaxLines: 2,
                  ),
                ),
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
