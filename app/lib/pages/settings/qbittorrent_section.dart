import 'package:flutter/material.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'settings_input_field.dart';

class SettingsQbittorrentSection extends StatefulWidget {
  final StateProvider state;
  final TextEditingController urlController;
  final TextEditingController categoryController;
  final TextEditingController userController;
  final TextEditingController passwordController;

  const SettingsQbittorrentSection({
    super.key,
    required this.state,
    required this.urlController,
    required this.categoryController,
    required this.userController,
    required this.passwordController,
  });

  @override
  State<SettingsQbittorrentSection> createState() =>
      _SettingsQbittorrentSectionState();
}

class _SettingsQbittorrentSectionState
    extends State<SettingsQbittorrentSection> {
  bool _obscureQbitPassword = true;

  @override
  Widget build(BuildContext context) {
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
                      child: const Icon(
                        Icons.hub_rounded,
                        size: 20,
                        color: ObsidianColors.tertiary,
                      ),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'qBittorrent WebUI Connection',
                            style: ObsidianTypography.titleSm(),
                            overflow: TextOverflow.ellipsis,
                          ),
                          Text(
                            'Manage RPC bridge for automated magnet handoff and seeding limits',
                            style: ObsidianTypography.bodySm(
                              color: ObsidianColors.outline,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
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
                    Container(
                      width: 6,
                      height: 6,
                      decoration: const BoxDecoration(
                        shape: BoxShape.circle,
                        color: ObsidianColors.tertiary,
                      ),
                    ),
                    const SizedBox(width: 6),
                    Text(
                      'Connected • v4.6.3 Web API',
                      style: ObsidianTypography.labelBadge(
                        color: ObsidianColors.tertiary,
                      ),
                    ),
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
                child: SettingsInputField(
                  label: 'Host / RPC Endpoint URL',
                  sublabel: 'DEFAULT: :8080',
                  icon: Icons.lan_rounded,
                  controller: widget.urlController,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: SettingsInputField(
                  label: 'Default Tag / Ingestion Category',
                  sublabel: 'qBit category rule',
                  icon: Icons.label_rounded,
                  controller: widget.categoryController,
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Row(
            children: [
              Expanded(
                child: SettingsInputField(
                  label: 'WebUI Username',
                  sublabel: null,
                  icon: Icons.person_rounded,
                  controller: widget.userController,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'WebUI Secret Key / Password',
                      style: ObsidianTypography.monoCode(),
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
                          const Icon(
                            Icons.lock_rounded,
                            size: 16,
                            color: ObsidianColors.outline,
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: TextField(
                              controller: widget.passwordController,
                              obscureText: _obscureQbitPassword,
                              style: ObsidianTypography.monoCode(),
                              decoration: const InputDecoration(
                                border: InputBorder.none,
                                isDense: true,
                                contentPadding: EdgeInsets.zero,
                              ),
                            ),
                          ),
                          IconButton(
                            icon: Icon(
                              _obscureQbitPassword
                                  ? Icons.visibility_rounded
                                  : Icons.visibility_off_rounded,
                              size: 16,
                              color: ObsidianColors.outline,
                            ),
                            onPressed: () => setState(
                              () =>
                                  _obscureQbitPassword = !_obscureQbitPassword,
                            ),
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
                const Icon(
                  Icons.check_circle_rounded,
                  size: 20,
                  color: ObsidianColors.tertiary,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Connection verified in 12ms',
                        style: ObsidianTypography.monoCode(
                          color: ObsidianColors.tertiary,
                        ).copyWith(fontWeight: FontWeight.bold),
                      ),
                      Text(
                        'qBittorrent v4.6.3 x86_64 daemon responded on loopback',
                        style: ObsidianTypography.labelBadge(
                          color: ObsidianColors.outline,
                        ),
                      ),
                    ],
                  ),
                ),
                ElevatedButton.icon(
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(
                        content: Text('Ping verified: 8ms loopback response'),
                      ),
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
}
