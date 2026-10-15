import 'package:flutter/material.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';

class DecisionCardFrame extends StatelessWidget {
  final Color accentColor;
  final String categoryTag;
  final IconData tagIcon;
  final Color tagColor;
  final String metaText;
  final String title;
  final String description;
  final Widget content;
  final String autoRuleText;
  final String ruleId;
  final VoidCallback? onSecondaryAction;
  final String? secondaryActionLabel;

  const DecisionCardFrame({
    super.key,
    required this.accentColor,
    required this.categoryTag,
    required this.tagIcon,
    required this.tagColor,
    required this.metaText,
    required this.title,
    required this.description,
    required this.content,
    required this.autoRuleText,
    required this.ruleId,
    this.onSecondaryAction,
    this.secondaryActionLabel,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      clipBehavior: Clip.antiAlias,
      child: Stack(
        children: [
          Positioned(
            left: 0,
            top: 0,
            bottom: 0,
            width: 4,
            child: Container(color: accentColor),
          ),
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                // Header Row
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Container(
                                padding: const EdgeInsets.symmetric(
                                    horizontal: 6, vertical: 2),
                                decoration: BoxDecoration(
                                    color: tagColor.withOpacity(0.2),
                                    borderRadius: BorderRadius.circular(4)),
                                child: Row(
                                  mainAxisSize: MainAxisSize.min,
                                  children: [
                                    Icon(tagIcon, size: 12, color: tagColor),
                                    const SizedBox(width: 4),
                                    Text(categoryTag,
                                        style: ObsidianTypography.labelBadge(
                                            color: tagColor, fontSize: 9)),
                                  ],
                                ),
                              ),
                              const SizedBox(width: 8),
                              Text(metaText,
                                  style: ObsidianTypography.monoCode(
                                      color: ObsidianColors.outline,
                                      fontSize: 10)),
                            ],
                          ),
                          const SizedBox(height: 6),
                          Text(title,
                              style:
                                  ObsidianTypography.headlineMd(fontSize: 16)),
                          const SizedBox(height: 2),
                          Text(description,
                              style: ObsidianTypography.bodySm(
                                  color: ObsidianColors.onSurfaceVariant,
                                  fontSize: 11)),
                        ],
                      ),
                    ),
                    if (secondaryActionLabel != null)
                      OutlinedButton(
                        onPressed: onSecondaryAction,
                        style: OutlinedButton.styleFrom(
                          side: const BorderSide(color: ObsidianColors.border),
                          padding: const EdgeInsets.symmetric(
                              horizontal: 10, vertical: 6),
                        ),
                        child: Text(secondaryActionLabel!,
                            style: ObsidianTypography.monoCode(
                                color: ObsidianColors.error, fontSize: 10)),
                      ),
                  ],
                ),
                const SizedBox(height: 14),

                // Custom Body Content
                content,
                const SizedBox(height: 14),

                // Footer with "Always do this" auto-rule checkbox
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                  decoration: BoxDecoration(
                    color:
                        ObsidianColors.surfaceContainerLowest.withOpacity(0.5),
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Expanded(
                        child: Row(
                          children: [
                            const Icon(Icons.check_box_outline_blank_rounded,
                                size: 16, color: ObsidianColors.primary),
                            const SizedBox(width: 6),
                            Expanded(
                              child: Text(
                                autoRuleText,
                                style: ObsidianTypography.bodySm(
                                    color: ObsidianColors.onSurfaceVariant,
                                    fontSize: 10),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                          ],
                        ),
                      ),
                      const SizedBox(width: 8),
                      Text('Rule ID: $ruleId',
                          style: ObsidianTypography.monoCode(
                              color: ObsidianColors.outline, fontSize: 9)),
                    ],
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
