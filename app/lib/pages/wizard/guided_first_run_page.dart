import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import '../../widgets/loading/ingestion_progress.dart';
import 'guided_first_run_help_dialog.dart';
import 'guided_folders_screen.dart';
import 'guided_mal_screen.dart';
import 'guided_qbittorrent_screen.dart';
import 'guided_welcome_screen.dart';
import 'step_quality_groups.dart';
import 'step_review_prelaunch.dart';
import 'wizard_plan_confirmation.dart';

/// Guided First Run Experience for Windows.
///
/// Walks users step-by-step through:
/// 1. Welcome & Prerequisites
/// 2. qBittorrent Web UI detection & setup
/// 3. MyAnimeList connection (OAuth / Guest)
/// 4. Folders & cross-drive hardlink warning
/// 5. Preferences, Review & Real Ingestion Pipeline
class GuidedFirstRunPage extends StatefulWidget {
  final VoidCallback onComplete;
  final int initialStep;

  const GuidedFirstRunPage({
    super.key,
    required this.onComplete,
    this.initialStep = 0,
  });

  @override
  State<GuidedFirstRunPage> createState() => _GuidedFirstRunPageState();
}

class _GuidedFirstRunPageState extends State<GuidedFirstRunPage> {
  late int _currentStep;
  bool _isIngesting = false;
  bool _automationOn = false;
  bool _bypassDailyCap = false;

  final _malUserController = TextEditingController(text: 'akumasan2');
  final _malClientIdController = TextEditingController();
  final _qbitUrlController = TextEditingController(
    text: 'http://127.0.0.1:8080',
  );
  final _qbitUserController = TextEditingController(text: 'admin');
  final _qbitPasswordController = TextEditingController();
  final _libraryController = TextEditingController();
  final _downloadController = TextEditingController();

  static const List<String> _stepTitles = [
    '1. Welcome',
    '2. qBittorrent',
    '3. MyAnimeList',
    '4. Folders',
    '5. Review & Launch',
  ];

  @override
  void initState() {
    super.initState();
    _currentStep = widget.initialStep;
    WidgetsBinding.instance.addPostFrameCallback((_) async {
      if (mounted) {
        final state = context.read<StateProvider>();
        await state.refreshSettings();
        await state.refreshStatus();
        if (!mounted) return;
        final savedUrl = state.settings['qbit_url'] as String?;
        if (savedUrl != null && savedUrl.isNotEmpty) {
          _qbitUrlController.text = savedUrl;
        }
        if (state.malUsername != null && state.malUsername!.isNotEmpty) {
          _malUserController.text = state.malUsername!;
        }
        if (state.libraryFolder.isNotEmpty) {
          _libraryController.text = state.libraryFolder;
        }
      }
    });
  }

  @override
  void dispose() {
    _malUserController.dispose();
    _malClientIdController.dispose();
    _qbitUrlController.dispose();
    _qbitUserController.dispose();
    _qbitPasswordController.dispose();
    _libraryController.dispose();
    _downloadController.dispose();
    super.dispose();
  }

  Future<void> _startIngestion(
    StateProvider state, {
    required bool automationOn,
    IngestionStage? resumeFrom,
  }) async {
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

  void _nextStep() {
    if (_currentStep < 4) {
      setState(() => _currentStep += 1);
    }
  }

  void _prevStep() {
    if (_currentStep > 0) {
      setState(() => _currentStep -= 1);
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

    final isIngestingOrFailed =
        _isIngesting ||
        state.isIngesting ||
        state.ingestionStage == IngestionStage.failed;

    return Scaffold(
      backgroundColor: ObsidianColors.surface,
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 900),
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
                  // Header with Step indicator & Help button
                  Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 24,
                      vertical: 18,
                    ),
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Flexible(
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              const Icon(
                                Icons.rocket_launch_rounded,
                                color: ObsidianColors.primary,
                                size: 22,
                              ),
                              const SizedBox(width: 10),
                              Flexible(
                                child: Text(
                                  'Genrl Setup Guide',
                                  style: ObsidianTypography.headlineMd(),
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                              const SizedBox(width: 12),
                              Container(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 8,
                                  vertical: 3,
                                ),
                                decoration: BoxDecoration(
                                  color: ObsidianColors.surfaceContainerHigh,
                                  borderRadius: BorderRadius.circular(6),
                                ),
                                child: Text(
                                  _stepTitles[_currentStep],
                                  style: ObsidianTypography.labelBadge(
                                    color: ObsidianColors.primary,
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ),
                        Row(
                          children: [
                            TextButton.icon(
                              onPressed: () =>
                                  GuidedFirstRunHelpDialog.show(context),
                              icon: const Icon(
                                Icons.help_outline_rounded,
                                size: 16,
                              ),
                              label: const Text('Help & Troubleshooting'),
                            ),
                            const SizedBox(width: 8),
                            IconButton(
                              icon: const Icon(Icons.close_rounded, size: 20),
                              tooltip: 'Exit setup',
                              onPressed: widget.onComplete,
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),

                  // Progress Bar
                  LinearProgressIndicator(
                    value: (_currentStep + 1) / 5.0,
                    backgroundColor: ObsidianColors.surfaceContainerHighest,
                    valueColor: const AlwaysStoppedAnimation<Color>(
                      ObsidianColors.primary,
                    ),
                    minHeight: 3,
                  ),

                  // Scratch DB badge if set
                  if (state.bridge.customDataDir != null)
                    Container(
                      width: double.infinity,
                      padding: const EdgeInsets.symmetric(
                        horizontal: 16,
                        vertical: 6,
                      ),
                      color: ObsidianColors.tertiary.withOpacity(0.12),
                      child: Text(
                        'SCRATCH DB: ${state.bridge.customDataDir}',
                        style: ObsidianTypography.labelBadge(
                          color: ObsidianColors.tertiary,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),

                  // Main Step Body
                  Padding(
                    padding: const EdgeInsets.all(24),
                    child: isIngestingOrFailed
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
                        : _buildStepContent(state),
                  ),

                  // Bottom Action Strip
                  if (!isIngestingOrFailed)
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 24,
                        vertical: 16,
                      ),
                      decoration: const BoxDecoration(
                        color: ObsidianColors.surfaceContainer,
                        borderRadius: BorderRadius.only(
                          bottomLeft: Radius.circular(16),
                          bottomRight: Radius.circular(16),
                        ),
                        border: Border(
                          top: BorderSide(color: ObsidianColors.border),
                        ),
                      ),
                      child: Wrap(
                        alignment: WrapAlignment.spaceBetween,
                        crossAxisAlignment: WrapCrossAlignment.center,
                        spacing: 12,
                        runSpacing: 10,
                        children: [
                          if (_currentStep > 0)
                            OutlinedButton.icon(
                              onPressed: _prevStep,
                              icon: const Icon(
                                Icons.arrow_back_rounded,
                                size: 16,
                              ),
                              label: const Text('Back'),
                            ),
                          if (_currentStep < 4)
                            ElevatedButton.icon(
                              onPressed: _nextStep,
                              icon: const Icon(
                                Icons.arrow_forward_rounded,
                                size: 16,
                              ),
                              label: const Text('Continue'),
                            )
                          else
                            Wrap(
                              spacing: 10,
                              runSpacing: 8,
                              children: [
                                OutlinedButton.icon(
                                  key: const Key(
                                    'guided_start_automation_off_btn',
                                  ),
                                  onPressed: () => _startIngestion(
                                    state,
                                    automationOn: false,
                                  ),
                                  icon: const Icon(
                                    Icons.pause_circle_outline,
                                    size: 18,
                                  ),
                                  label: const Text(
                                    'Start with automation off',
                                  ),
                                ),
                                ElevatedButton.icon(
                                  key: const Key(
                                    'guided_start_automation_on_btn',
                                  ),
                                  onPressed: () => _startIngestion(
                                    state,
                                    automationOn: true,
                                  ),
                                  icon: const Icon(Icons.bolt, size: 18),
                                  label: const Text('Turn automation on'),
                                ),
                              ],
                            ),
                        ],
                      ),
                    ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildStepContent(StateProvider state) {
    switch (_currentStep) {
      case 0:
        return GuidedWelcomeScreen(onNext: _nextStep);
      case 1:
        return GuidedQbittorrentScreen(
          urlController: _qbitUrlController,
          userController: _qbitUserController,
          passwordController: _qbitPasswordController,
          state: state,
        );
      case 2:
        return GuidedMalScreen(
          userController: _malUserController,
          clientIdController: _malClientIdController,
          state: state,
        );
      case 3:
        return GuidedFoldersScreen(
          libraryController: _libraryController,
          downloadController: _downloadController,
          state: state,
        );
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
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const StepQualityGroups(),
            const SizedBox(height: 24),
            StepReviewPrelaunch(
              state: state,
              qbitUrlController: _qbitUrlController,
              onRecheckQbit: () => state.refreshStatus(),
              onOpenQbitSettings: () => setState(() => _currentStep = 1),
            ),
          ],
        );
    }
  }
}
