import 'dart:async';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../widgets/loading/ingestion_progress.dart';
import 'step_mal_account.dart';
import 'step_qbittorrent.dart';
import 'step_quality_groups.dart';
import 'step_review_prelaunch.dart';
import 'step_root_path.dart';
import 'wizard_header.dart';
import 'wizard_plan_confirmation.dart';
import 'wizard_stepper.dart';

class WizardPage extends StatefulWidget {
  final VoidCallback onComplete;
  final int initialStep;
  final String? initialQbitUrl;
  final String? initialMalUsername;

  const WizardPage({
    super.key,
    required this.onComplete,
    this.initialStep = 0,
    this.initialQbitUrl,
    this.initialMalUsername,
  });

  @override
  State<WizardPage> createState() => _WizardPageState();
}

class _WizardPageState extends State<WizardPage> {
  late int _currentStep;
  bool _bypassDailyCap = false;
  bool _isIngesting = false;
  bool _automationOn = false;
  bool _loadingSettings = true;
  bool _savingStep = false;
  String? _stepError;

  Timer? _preflightTimer;

  late final _malUserController = TextEditingController(
    text: widget.initialMalUsername ?? 'akumasan2',
  );
  final _malClientIdController = TextEditingController();
  late final _qbitUrlController = TextEditingController(
    text: widget.initialQbitUrl ?? '',
  );
  final _qbitUserController = TextEditingController(text: 'admin');
  final _qbitPasswordController = TextEditingController();
  final _rootPathController = TextEditingController();

  void _startPreflightPolling() {
    _preflightTimer?.cancel();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && _currentStep == 4) {
        context.read<StateProvider>().refreshStatus();
      }
    });
    _preflightTimer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (mounted && _currentStep == 4) {
        context.read<StateProvider>().refreshStatus();
      }
    });
  }

  @override
  void initState() {
    super.initState();
    _currentStep = widget.initialStep;
    if (_currentStep == 4) {
      _startPreflightPolling();
    }
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      if (mounted) {
        final state = context.read<StateProvider>();
        await state.refreshSettings();
        await state.refreshStatus();
        if (!mounted) return;
        final savedUrl = state.settings['qbit_url'] as String?;
        if (widget.initialQbitUrl == null &&
            _qbitUrlController.text.isEmpty &&
            savedUrl != null) {
          _qbitUrlController.text = savedUrl;
        }
        if (state.malUsername != null && state.malUsername!.isNotEmpty) {
          _malUserController.text = state.malUsername!;
        }
        if (state.libraryFolder.isNotEmpty) {
          _rootPathController.text = state.libraryFolder;
        }
        setState(() => _loadingSettings = false);
      }
    });
  }

  @override
  void dispose() {
    _preflightTimer?.cancel();
    _malUserController.dispose();
    _malClientIdController.dispose();
    _qbitUrlController.dispose();
    _qbitUserController.dispose();
    _qbitPasswordController.dispose();
    _rootPathController.dispose();
    super.dispose();
  }

  Future<void> _startIngestion(
    StateProvider state, {
    required bool automationOn,
    IngestionStage? resumeFrom,
  }) async {
    _preflightTimer?.cancel();
    _automationOn = automationOn;
    setState(() => _isIngesting = true);
    await state.startIngestion(
      automationOn: automationOn,
      bypassDailyCap: _bypassDailyCap,
      resumeFrom: resumeFrom ?? IngestionStage.malSync,
    );
    if (mounted) {
      setState(() => _isIngesting = false);
      if (state.ingestionStage == IngestionStage.completed) {
        widget.onComplete();
      }
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
                  WizardHeader(
                    currentStep: _currentStep,
                    onClose: widget.onComplete,
                  ),
                  if (state.bridge.customDataDir != null)
                    Container(
                      width: double.infinity,
                      padding: const EdgeInsets.symmetric(
                        horizontal: 16,
                        vertical: 8,
                      ),
                      color: ObsidianColors.tertiary.withOpacity(0.15),
                      child: Row(
                        children: [
                          const Icon(
                            Icons.science_outlined,
                            size: 16,
                            color: ObsidianColors.tertiary,
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: Text(
                              'SCRATCH DB: ${state.bridge.customDataDir}',
                              style: ObsidianTypography.labelBadge(
                                color: ObsidianColors.tertiary,
                              ),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ],
                      ),
                    ),
                  WizardStepper(currentStep: _currentStep, state: state),
                  if (_stepError != null)
                    Padding(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 24,
                        vertical: 8,
                      ),
                      child: Text(
                        _stepError!,
                        style: const TextStyle(color: ObsidianColors.error),
                      ),
                    ),
                  Padding(
                    padding: const EdgeInsets.all(24),
                    child:
                        (_isIngesting ||
                            state.isIngesting ||
                            state.ingestionStage == IngestionStage.failed)
                        ? IngestionProgress(
                            stage: state.ingestionStage,
                            failedStage: state.failedStage,
                            processedCount: state.ingestionProcessedCount,
                            totalCount: state.ingestionTotalCount,
                            etaSeconds: state.ingestionEtaSeconds,
                            isPaused: state.isIngestionPaused,
                            errorMessage: state.ingestionError,
                            dbPath: state.bridge.customDataDir,
                            jikanBaseUrl: state.jikanBaseUrl,
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
                            onRetry: () => _startIngestion(
                              state,
                              automationOn: _automationOn,
                              resumeFrom:
                                  state.failedStage ?? IngestionStage.malSync,
                            ),
                          )
                        : _buildStepBody(state),
                  ),
                  if (!_isIngesting &&
                      !state.isIngesting &&
                      state.ingestionStage != IngestionStage.failed)
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
        return StepRootPath(pathController: _rootPathController, state: state);
      case 3:
        return const StepQualityGroups();
      case 4:
      default:
        if (state.isPlanReadyAwaitingConfirm) {
          return WizardPlanConfirmation(
            state: state,
            bypassDailyCap: _bypassDailyCap,
            onToggleBypass: (val) => setState(() => _bypassDailyCap = val),
            onConfirm: () => _startIngestion(
              state,
              automationOn: _automationOn,
              resumeFrom: IngestionStage.createJobs,
            ),
            onBack: () => setState(() {
              state.isPlanReadyAwaitingConfirm = false;
            }),
          );
        }
        return StepReviewPrelaunch(
          state: state,
          qbitUrlController: _qbitUrlController,
          onRecheckQbit: () => state.refreshStatus(),
          onOpenQbitSettings: () {
            setState(() {
              _currentStep = 1;
              _preflightTimer?.cancel();
            });
          },
        );
    }
  }

  Future<void> _advanceStep(StateProvider state) async {
    if (_currentStep == 1) {
      setState(() {
        _savingStep = true;
        _stepError = null;
      });
      final saved = await state.saveQbitEndpoint(_qbitUrlController.text);
      if (!mounted) return;
      if (!saved) {
        setState(() {
          _savingStep = false;
          _stepError =
              'Could not save qBittorrent endpoint. Check the URL and retry.';
        });
        return;
      }
    }
    if (_currentStep == 0) {
      final user = _malUserController.text.trim();
      if (user.isNotEmpty) {
        state.bridge.execute(['mal', 'public-profile', user]);
      }
    }
    if (_currentStep == 2) {
      final root = _rootPathController.text.trim();
      if (root.isNotEmpty) {
        state.setLibraryFolder(root);
        state.bridge.setSetting('library_folder', root);
      }
    }
    setState(() {
      _savingStep = false;
      _currentStep++;
      if (_currentStep == 4) _startPreflightPolling();
    });
  }

  Widget _buildBottomActionStrip(StateProvider state) {
    if (state.isPlanReadyAwaitingConfirm) {
      return const SizedBox.shrink();
    }
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
            key: const Key('wizard_back_btn'),
            onPressed: !_savingStep && _currentStep > 0
                ? () {
                    setState(() {
                      _currentStep--;
                      if (_currentStep != 4) {
                        _preflightTimer?.cancel();
                      }
                    });
                  }
                : null,
            style: OutlinedButton.styleFrom(
              side: const BorderSide(color: ObsidianColors.border),
              foregroundColor: ObsidianColors.onSurfaceVariant,
            ),
            child: const Text('Back'),
          ),
          if (!isLastStep)
            ElevatedButton(
              key: const Key('wizard_next_btn'),
              onPressed: _loadingSettings || _savingStep
                  ? null
                  : () => _advanceStep(state),
              style: ElevatedButton.styleFrom(
                backgroundColor: ObsidianColors.primary,
                foregroundColor: ObsidianColors.onPrimary,
              ),
              child: Text(_savingStep ? 'Checking connection…' : 'Next Step'),
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
