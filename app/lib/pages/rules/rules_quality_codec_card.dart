import 'package:flutter/material.dart';

import '../../theme/colors.dart';
import '../../theme/typography.dart';

class RulesQualityCodecCard extends StatelessWidget {
  final String targetResolution;
  final ValueChanged<String> onResolutionChanged;
  final bool hevcAllowed;
  final ValueChanged<bool> onHevcChanged;
  final bool avcAllowed;
  final ValueChanged<bool> onAvcChanged;
  final bool av1Allowed;
  final ValueChanged<bool> onAv1Changed;
  final bool mpegAllowed;
  final ValueChanged<bool> onMpegChanged;
  final bool flacAudio;
  final ValueChanged<bool> onFlacChanged;
  final bool opusAudio;
  final ValueChanged<bool> onOpusChanged;

  const RulesQualityCodecCard({
    super.key,
    required this.targetResolution,
    required this.onResolutionChanged,
    required this.hevcAllowed,
    required this.onHevcChanged,
    required this.avcAllowed,
    required this.onAvcChanged,
    required this.av1Allowed,
    required this.onAv1Changed,
    required this.mpegAllowed,
    required this.onMpegChanged,
    required this.flacAudio,
    required this.onFlacChanged,
    required this.opusAudio,
    required this.onOpusChanged,
  });

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
          Wrap(
            alignment: WrapAlignment.spaceBetween,
            crossAxisAlignment: WrapCrossAlignment.center,
            spacing: 8,
            runSpacing: 4,
            children: [
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(
                    Icons.tune_rounded,
                    size: 20,
                    color: ObsidianColors.primary,
                  ),
                  const SizedBox(width: 8),
                  Text(
                    'Quality & Codec Specs',
                    style: ObsidianTypography.titleSm(),
                  ),
                ],
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  'v2.4 Core',
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.outline,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 14),

          // Target Resolution Segmented Control
          Text(
            'TARGET RESOLUTION',
            style: ObsidianTypography.labelBadge(color: ObsidianColors.outline),
          ),
          const SizedBox(height: 6),
          Row(
            children: [
              _buildResolutionButton('720p HD', '720p'),
              const SizedBox(width: 6),
              _buildResolutionButton('1080p FHD', '1080p'),
              const SizedBox(width: 6),
              _buildResolutionButton('2160p 4K', '2160p'),
            ],
          ),
          const SizedBox(height: 4),
          Text(
            'Optimal profile: 1080p SDR/HDR downsampled with dual audio preferred.',
            style: ObsidianTypography.monoCode(color: ObsidianColors.secondary),
          ),
          const SizedBox(height: 14),

          // Allowed Video Codecs Checkbox Grid
          Text(
            'ALLOWED VIDEO CODECS',
            style: ObsidianTypography.labelBadge(color: ObsidianColors.outline),
          ),
          const SizedBox(height: 6),
          Row(
            children: [
              Expanded(
                child: _buildCheckboxTile(
                  title: 'HEVC (H.265)',
                  subtitle: 'Archival priority',
                  value: hevcAllowed,
                  onChanged: (v) => onHevcChanged(v ?? true),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: _buildCheckboxTile(
                  title: 'AVC (H.264)',
                  subtitle: 'Universal compat',
                  value: avcAllowed,
                  onChanged: (v) => onAvcChanged(v ?? true),
                ),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Row(
            children: [
              Expanded(
                child: _buildCheckboxTile(
                  title: 'AV1',
                  subtitle: 'Next-gen open',
                  value: av1Allowed,
                  onChanged: (v) => onAv1Changed(v ?? true),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: _buildCheckboxTile(
                  title: 'MPEG-2 / DivX',
                  subtitle: 'Legacy (Blocked)',
                  value: mpegAllowed,
                  onChanged: (v) => onMpegChanged(v ?? false),
                ),
              ),
            ],
          ),
          const SizedBox(height: 14),

          // Audio Channels Checkboxes
          Text(
            'AUDIO CHANNELS & CODECS',
            style: ObsidianTypography.labelBadge(color: ObsidianColors.outline),
          ),
          const SizedBox(height: 6),
          Wrap(
            spacing: 8,
            runSpacing: 6,
            children: [
              FilterChip(
                selected: flacAudio,
                label: const Text('Lossless FLAC (24-bit/5.1)'),
                onSelected: (v) => onFlacChanged(v),
                selectedColor: ObsidianColors.primary.withOpacity(0.2),
                checkmarkColor: ObsidianColors.primary,
              ),
              FilterChip(
                selected: opusAudio,
                label: const Text('Opus / AAC 2.0+'),
                onSelected: (v) => onOpusChanged(v),
                selectedColor: ObsidianColors.primary.withOpacity(0.2),
                checkmarkColor: ObsidianColors.primary,
              ),
            ],
          ),
          const SizedBox(height: 14),

          // File Size Limits Bar Visualizer
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                'EPISODE SIZE GATE',
                style: ObsidianTypography.labelBadge(
                  color: ObsidianColors.outline,
                ),
              ),
              Text(
                '400 MB — 1.80 GB',
                style: ObsidianTypography.monoCode(
                  color: ObsidianColors.primary,
                ).copyWith(fontWeight: FontWeight.bold),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Container(
            height: 6,
            width: double.infinity,
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(3),
            ),
            child: Row(
              children: [
                const Spacer(flex: 16),
                Expanded(
                  flex: 52,
                  child: Container(
                    decoration: BoxDecoration(
                      color: ObsidianColors.primary,
                      borderRadius: BorderRadius.circular(3),
                    ),
                  ),
                ),
                const Spacer(flex: 32),
              ],
            ),
          ),
          const SizedBox(height: 4),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                '100M',
                style: ObsidianTypography.labelBadge(
                  color: ObsidianColors.outline,
                ),
              ),
              Text(
                '400M (Min)',
                style: ObsidianTypography.labelBadge(
                  color: ObsidianColors.primary,
                ),
              ),
              Text(
                '1.0G',
                style: ObsidianTypography.labelBadge(
                  color: ObsidianColors.outline,
                ),
              ),
              Text(
                '1.8G (Max)',
                style: ObsidianTypography.labelBadge(
                  color: ObsidianColors.primary,
                ),
              ),
              Text(
                '3.5G+',
                style: ObsidianTypography.labelBadge(
                  color: ObsidianColors.outline,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildResolutionButton(String label, String value) {
    final isSelected = targetResolution == value;
    return Expanded(
      child: InkWell(
        onTap: () => onResolutionChanged(value),
        borderRadius: BorderRadius.circular(6),
        child: Container(
          padding: const EdgeInsets.symmetric(vertical: 8),
          decoration: BoxDecoration(
            color: isSelected
                ? ObsidianColors.primaryContainer
                : ObsidianColors.surfaceContainer,
            borderRadius: BorderRadius.circular(6),
            border: Border.all(
              color: isSelected
                  ? ObsidianColors.primary
                  : ObsidianColors.border,
            ),
          ),
          alignment: Alignment.center,
          child: Text(
            label,
            style: ObsidianTypography.monoCode(
              color: isSelected
                  ? ObsidianColors.onPrimaryContainer
                  : ObsidianColors.onSurfaceVariant,
            ).copyWith(fontWeight: FontWeight.bold),
          ),
        ),
      ),
    );
  }

  Widget _buildCheckboxTile({
    required String title,
    required String subtitle,
    required bool value,
    required ValueChanged<bool?> onChanged,
  }) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Row(
        children: [
          Checkbox(
            value: value,
            onChanged: onChanged,
            activeColor: ObsidianColors.primary,
          ),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: ObsidianTypography.bodySm().copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
                Text(
                  subtitle,
                  style: ObsidianTypography.labelBadge(
                    color: ObsidianColors.outline,
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
