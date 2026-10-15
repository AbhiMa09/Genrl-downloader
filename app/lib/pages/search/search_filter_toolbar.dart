import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class SearchFilterToolbar extends StatelessWidget {
  final String selectedRes;
  final ValueChanged<String> onSelectRes;
  final bool trustedOnly;
  final ValueChanged<bool> onToggleTrusted;

  const SearchFilterToolbar({
    super.key,
    required this.selectedRes,
    required this.onSelectRes,
    required this.trustedOnly,
    required this.onToggleTrusted,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Wrap(
        alignment: WrapAlignment.spaceBetween,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: 12,
        runSpacing: 8,
        children: [
          // Resolution Pills
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text('RES: ',
                  style: ObsidianTypography.labelBadge(
                      color: ObsidianColors.outline, fontSize: 9)),
              _buildResPill('All'),
              _buildResPill('4K UHD'),
              _buildResPill('1080p'),
              _buildResPill('720p'),
              _buildResPill('SD'),
            ],
          ),

          // Group & Swarm
          Wrap(
            spacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(6)),
                child: Row(
                  children: [
                    Text('GROUP: ',
                        style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline, fontSize: 8)),
                    Text('SubsPlease (Preferred)',
                        style: ObsidianTypography.monoCode(
                            color: ObsidianColors.primary, fontSize: 10)),
                  ],
                ),
              ),
              Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(6)),
                child: Row(
                  children: [
                    Text('SWARM: ',
                        style: ObsidianTypography.labelBadge(
                            color: ObsidianColors.outline, fontSize: 8)),
                    Text('Seeds ≥ 10',
                        style: ObsidianTypography.monoMetric(
                            color: ObsidianColors.tertiary, fontSize: 10)),
                  ],
                ),
              ),
              Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainer,
                    borderRadius: BorderRadius.circular(6)),
                child: Text('800 MB — 2.5 GB',
                    style: ObsidianTypography.monoCode(
                        color: ObsidianColors.onSurface, fontSize: 10)),
              ),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Checkbox(
                    value: trustedOnly,
                    onChanged: (val) => onToggleTrusted(val ?? true),
                    activeColor: ObsidianColors.primary,
                  ),
                  Text('Trusted / VIP Only',
                      style: ObsidianTypography.bodySm(fontSize: 11)),
                ],
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildResPill(String res) {
    final isSelected = selectedRes == res;
    return InkWell(
      onTap: () => onSelectRes(res),
      borderRadius: BorderRadius.circular(4),
      child: Container(
        margin: const EdgeInsets.only(right: 4),
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          color:
              isSelected ? ObsidianColors.primary : ObsidianColors.surfaceContainer,
          borderRadius: BorderRadius.circular(4),
        ),
        child: Text(
          res,
          style: ObsidianTypography.monoCode(
            color: isSelected
                ? ObsidianColors.onPrimary
                : ObsidianColors.outline,
            fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
            fontSize: 10,
          ),
        ),
      ),
    );
  }
}
