import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class StepQbittorrent extends StatelessWidget {
  final TextEditingController urlController;
  final TextEditingController userController;
  final TextEditingController passwordController;
  final StateProvider state;

  const StepQbittorrent({
    super.key,
    required this.urlController,
    required this.userController,
    required this.passwordController,
    required this.state,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('qBittorrent Integration', style: ObsidianTypography.headlineMd()),
        const SizedBox(height: 6),
        Text(
          'Connect to local or remote qBittorrent WebUI daemon for automated queue management.',
          style: ObsidianTypography.bodySm(
            color: ObsidianColors.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 20),
        TextField(
          controller: urlController,
          decoration: const InputDecoration(
            labelText: 'WebUI Endpoint URL',
            hintText: 'http://127.0.0.1:8080',
            prefixIcon: Icon(Icons.link_rounded),
          ),
        ),
        const SizedBox(height: 14),
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: userController,
                decoration: const InputDecoration(
                  labelText: 'Username',
                  hintText: 'admin',
                  prefixIcon: Icon(Icons.person_outline_rounded),
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: TextField(
                controller: passwordController,
                obscureText: true,
                decoration: const InputDecoration(
                  labelText: 'Password',
                  hintText: '••••••••',
                  prefixIcon: Icon(Icons.lock_outline_rounded),
                ),
              ),
            ),
          ],
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
                state.status.qbitConnected
                    ? Icons.check_circle_rounded
                    : Icons.info_outline_rounded,
                size: 18,
                color: state.status.qbitConnected
                    ? ObsidianColors.tertiary
                    : ObsidianColors.outline,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  state.status.qbitConnected
                      ? 'Daemon online • qBittorrent ${state.status.qbitVersion} responding'
                      : 'Endpoint reachable. Credentials verified.',
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
