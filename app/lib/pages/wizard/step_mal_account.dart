import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class StepMalAccount extends StatelessWidget {
  final TextEditingController userController;
  final TextEditingController clientIdController;
  final StateProvider state;

  const StepMalAccount({
    super.key,
    required this.userController,
    required this.clientIdController,
    required this.state,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Connect MyAnimeList Account',
          style: ObsidianTypography.headlineMd(),
        ),
        const SizedBox(height: 6),
        Text(
          'Sync your anime watching, plan-to-watch, and completed library states.',
          style: ObsidianTypography.bodySm(
            color: ObsidianColors.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 20),
        TextField(
          controller: userController,
          decoration: const InputDecoration(
            labelText: 'MAL Username (Guest mode / Jikan fallback)',
            hintText: 'e.g. your_mal_username',
            helperText: 'Guest mode: read-only, possibly slow (Jikan rate limits). Not default.',
            helperMaxLines: 2,
            prefixIcon: Icon(Icons.person_outline_rounded),
          ),
        ),
        const SizedBox(height: 14),
        TextField(
          controller: clientIdController,
          decoration: const InputDecoration(
            labelText: 'Client ID (Optional PKCE)',
            hintText: 'Leave empty to use public API gateway',
            prefixIcon: Icon(Icons.key_rounded),
          ),
        ),
        const SizedBox(height: 16),
        Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: ObsidianColors.border),
          ),
          child: Row(
            children: [
              Icon(
                state.isMalConnected
                    ? Icons.check_circle_rounded
                    : Icons.info_outline_rounded,
                size: 18,
                color: state.isMalConnected
                    ? ObsidianColors.tertiary
                    : ObsidianColors.primary,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  state.isMalConnected
                      ? 'Connected as ${state.malUsername ?? userController.text}'
                      : 'Unlinked — Guest mode (Jikan) is read-only and rate-limited. OAuth recommended.',
                  style: ObsidianTypography.bodySm(
                    color: ObsidianColors.onSurface,
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
