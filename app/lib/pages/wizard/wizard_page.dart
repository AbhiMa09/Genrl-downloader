import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../widgets/loading/ingestion_progress.dart';
import 'step_mal_account.dart';
import 'step_qbittorrent.dart';
import 'step_quality_groups.dart';
import 'step_root_path.dart';
import 'wizard_daily_cap_warning.dart';
import 'wizard_header.dart';
import 'wizard_headline_card.dart';
import 'wizard_series_manifest.dart';
import 'wizard_stepper.dart';

class WizardPage extends StatefulWidget {
  final VoidCallback onComplete;
  final int initialStep;
  final bool useLegacyLaunchButton;

  const WizardPage({
    super.key,
    required this.onComplete,
    this.initialStep = 4,
    this.useLegacyLaunchButton = false,
  });

  @override
  State<WizardPage> createState() => _WizardPageState();
}

class _WizardPageState extends State<WizardPage> {
  late int _currentStep;
  bool _bypassDailyCap = false;
  bool _isIngesting = false;

  final _malUserController = TextEditingController(text: 'ren_kurokawa');
  final _malClientIdController = TextEditingController();
  final _qbitUrlController = TextEditingController(text: 'http://127.0.0.1:8080');
  final _qbitUserController = TextEditingController(text: 'admin');
  final _qbitPasswordController = TextEditingController(text: 'super_secure_token');
  final _rootPathController = TextEditingController(text: '/mnt/anime/TV');

  @override
  void initState() {
    super.initState();
    _currentStep = widget.initialStep;
  }

  @override
  void dispose() {
    _malUserController.dispose();
    _malClientIdController.dispose();
    _qbitUrlController.dispose();
    _qbitUserController.dispose();
    _qbitPasswordController.dispose();
    _rootPathController.dispose();
    super.dispose();
  }

  Future<void> _startIngestion(StateProvider state, {required bool automationOn}) async {
    setState(() => _isIngesting = true);
    await state.startIngestion(
      automationOn: automationOn,
      bypassDailyCap: _bypassDailyCap,
    );
    if (mounted && state.ingestionStage == IngestionStage.completed) {
      widget.onComplete();
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    if (state.isIngesting || _isIngesting) {
      if (state.ingestionStage == IngestionStage.completed) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted) widget.onComplete();
        });
      }
    }

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
                  WizardHeader(onClose: widget.onComplete),
                  WizardStepper(currentStep: _currentStep),
                  Padding(
                    padding: const EdgeInsets.all(24),
                    child: (_isIngesting || state.isIngesting)
                        ? IngestionProgress(
                            stage: state.ingestionStage,
                            processedCount: state.ingestionProcessedCount,
                            totalCount: state.ingestionTotalCount,
                            etaSeconds: state.ingestionEtaSeconds,
                            isPaused: state.isIngestionPaused,
                            errorMessage: state.ingestionError,
                            onPauseToggle: () {
                              if (state.isIngestionPaused) {
                                state.resumeIngestion();
                              } else {
                                state.pauseIngestion();
                              }
                            },
                            onBackground: () {
                              state.backgroundIngestion();
                              widget.onComplete();
                            },
                            onViewTelemetry: () {
                              widget.onComplete();
                              state.selectPage(5);
                            },
                            onRetry: () => _startIngestion(state, automationOn: true),
                          )
                        : _buildStepBody(state),
                  ),
                  if (!_isIngesting && !state.isIngesting)
                    _buildBottomActionStrip(state),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildStepBody(StateProvider state) {
    switch (_currentStep) {
      case 0:
        return StepMalAccount(
          userController: _malUserController,
          clientIdController: _malClientIdController,
          state: state,
        );
      case 1:
        return StepQbittorrent(
          urlController: _qbitUrlController,
          userController: _qbitUserController,
          passwordController: _qbitPasswordController,
          state: state,
        );
      case 2:
        return StepRootPath(
          pathController: _rootPathController,
          state: state,
        );
      case 3:
        return const StepQualityGroups();
      case 4:
      default:
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const WizardHeadlineCard(),
            const SizedBox(height: 16),
            WizardDailyCapWarning(
              bypassDailyCap: _bypassDailyCap,
              onToggleBypass: (val) => setState(() => _bypassDailyCap = val),
            ),
            const SizedBox(height: 16),
            WizardSeriesManifest(state: state),
          ],
        );
    }
  }

  Widget _buildBottomActionStrip(StateProvider state) {
    final isLastStep = _currentStep == 4;

    return Container(
      padding: const EdgeInsets.all(20),
      decoration: const BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.vertical(bottom: Radius.circular(16)),
      ),
      child: Wrap(
        alignment: WrapAlignment.spaceBetween,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: 12,
        runSpacing: 10,
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
          if (!isLastStep)
            ElevatedButton(
              onPressed: () {
                setState(() => _currentStep++);
              },
              style: ElevatedButton.styleFrom(
                backgroundColor: ObsidianColors.primary,
                foregroundColor: ObsidianColors.onPrimary,
              ),
              child: const Text('Next Step'),
            )
          else if (widget.useLegacyLaunchButton)
            ElevatedButton.icon(
              onPressed: widget.onComplete,
              icon: const Icon(Icons.rocket_launch_rounded, size: 18),
              label: const Text('Confirm & Launch Pipeline'),
              style: ElevatedButton.styleFrom(
                backgroundColor: ObsidianColors.primaryContainer,
                foregroundColor: ObsidianColors.onPrimaryContainer,
                padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
              ),
            )
          else
            Wrap(
              spacing: 10,
              runSpacing: 8,
              children: [
                OutlinedButton.icon(
                  key: const Key('start_automation_off_btn'),
                  onPressed: () => _startIngestion(state, automationOn: false),
                  icon: const Icon(Icons.pause_circle_outline, size: 18),
                  label: const Text('Start with automation off'),
                  style: OutlinedButton.styleFrom(
                    side: const BorderSide(color: ObsidianColors.border),
                    foregroundColor: ObsidianColors.onSurface,
                  ),
                ),
                ElevatedButton.icon(
                  key: const Key('start_automation_on_btn'),
                  onPressed: () => _startIngestion(state, automationOn: true),
                  icon: const Icon(Icons.bolt, size: 18),
                  label: const Text('Turn automation on'),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: ObsidianColors.primary,
                    foregroundColor: ObsidianColors.onPrimary,
                  ),
                ),
              ],
            ),
        ],
      ),
    );
  }
}
