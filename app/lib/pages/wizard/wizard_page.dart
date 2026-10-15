import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import 'wizard_daily_cap_warning.dart';
import 'wizard_header.dart';
import 'wizard_headline_card.dart';
import 'wizard_series_manifest.dart';
import 'wizard_stepper.dart';

class WizardPage extends StatefulWidget {
  final VoidCallback onComplete;

  const WizardPage({super.key, required this.onComplete});

  @override
  State<WizardPage> createState() => _WizardPageState();
}

class _WizardPageState extends State<WizardPage> {
  int _currentStep = 4; // Step 5: Review & Launch (0-indexed: 4)
  bool _bypassDailyCap = false;

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    return Scaffold(
      backgroundColor: ObsidianColors.surface,
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 890),
            child: Container(
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLow,
                borderRadius: BorderRadius.circular(16),
                border: Border.all(color: ObsidianColors.border),
                boxShadow: const [
                  BoxShadow(
                    color: Colors.black54,
                    blurRadius: 30,
                    offset: Offset(0, 10),
                  ),
                ],
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // Dialog Header Section
                  WizardHeader(onClose: widget.onComplete),

                  // 5-Step Horizontal Progress Stepper
                  WizardStepper(currentStep: _currentStep),

                  // Main Scrollable Body
                  Padding(
                    padding: const EdgeInsets.all(24),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        // Bento Headline Metric Card
                        const WizardHeadlineCard(),
                        const SizedBox(height: 16),

                        // Daily Cap Warning Box
                        WizardDailyCapWarning(
                          bypassDailyCap: _bypassDailyCap,
                          onToggleBypass: (val) =>
                              setState(() => _bypassDailyCap = val),
                        ),
                        const SizedBox(height: 16),

                        // Expandable Ingestion Preview List
                        WizardSeriesManifest(state: state),
                      ],
                    ),
                  ),

                  // Bottom Action Strip
                  _buildBottomActionStrip(),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildBottomActionStrip() {
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: const BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.vertical(bottom: Radius.circular(16)),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          OutlinedButton(
            onPressed: () {
              if (_currentStep > 0) {
                setState(() => _currentStep--);
              }
            },
            style: OutlinedButton.styleFrom(
              side: const BorderSide(color: ObsidianColors.border),
              foregroundColor: ObsidianColors.onSurfaceVariant,
            ),
            child: const Text('Back'),
          ),
          ElevatedButton.icon(
            onPressed: () {
              widget.onComplete();
            },
            icon: const Icon(Icons.rocket_launch_rounded, size: 18),
            label: const Text('Confirm & Launch Pipeline'),
            style: ElevatedButton.styleFrom(
              backgroundColor: ObsidianColors.primaryContainer,
              foregroundColor: ObsidianColors.onPrimaryContainer,
              padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
            ),
          ),
        ],
      ),
    );
  }
}
