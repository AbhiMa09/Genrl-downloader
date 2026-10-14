import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/state_provider.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';

class WizardPage extends StatefulWidget {
  final VoidCallback onComplete;

  const WizardPage({super.key, required this.onComplete});

  @override
  State<WizardPage> createState() => _WizardPageState();
}

class _WizardPageState extends State<WizardPage> {
  int _currentStep = 4; // Step 5: Review & Launch (0-indexed: 4)
  bool _bypassDailyCap = false;

  final List<Map<String, dynamic>> _detectedSeries = [
    {
      'title': 'Sousou no Frieren',
      'tier': 'Tier 1',
      'missing': 'Missing: Ep 21-28 (8 eps)',
      'spec': '[SubsPlease] 1080p HEVC',
      'size': '9.6 GB',
      'status': 'Ready to queue',
      'poster': 'https://lh3.googleusercontent.com/aida-public/AB6AXuDfpe9q8TwxbChTXt62KRnfzT6P0q5oUyRnhOJz5XU8rxP2d1dKAdL1BWECbDeu6FAiLfXNVQJ4x4yvMt1k3RmUWjP9ZYMNd5_vzLE71htdXjtwyLRZKCdtSqKau_ultPGac1Nt9FgNbQkvCLpu44LaBSBKhMiy5eu2zc8a6sXPdmsoID2gE_pzALbjE_gWto3XOvZzhsQGCrONZbf7IZp22SEUSYpsRB5ZED7VHtmX8ifFR3nAnL7x',
    },
    {
      'title': 'Dungeon Meshi',
      'tier': 'Tier 2',
      'missing': 'Missing: Ep 18-24 (7 eps)',
      'spec': '[Erai-raws] 1080p AV1',
      'size': '6.2 GB',
      'status': 'Ready to queue',
      'poster': 'https://lh3.googleusercontent.com/aida-public/AB6AXuAJHJTb5RHni39JaqXK_dJXf541U-A1mMEiMhBANVt2weWVgQadDAWTEs--cHelCu8kX92JMrdVsww33jwOHXsPUheluv_2ZmpBXUpPbuZ42bPkac6hUpsCJk7q0IrS4zTXeZrWEBG0doV4TeeGYFwE8quZ1Xh-YHTP846dcPYgvTbambsDwoF-wg8GZjaFDosKA7OKrvhimOnbGZWxRk_dxC_Es-RF2iFyOxmcFmkkq-NgQkhrXg4s',
    },
    {
      'title': 'Solo Leveling',
      'tier': 'Tier 1',
      'missing': 'Missing: Ep 09-12 (4 eps)',
      'spec': '[Judas] 1080p HEVC',
      'size': '4.1 GB',
      'status': 'Ready to queue',
      'poster': 'https://lh3.googleusercontent.com/aida-public/AB6AXuDGAYxtwLt1fEpEGn5Cykjf68bdazu7QWY07jO9onzSWzMhyqNHg4_AAW3j_E2cc5EYnvl5BNjAsXbk9Bs-7dyvAGv3siBjCMz8jQ0VHr2l6RB7YZt8i0CqG_Z2aZKQGvh4f2WwyA2Fx-zBSLf2lsOGFcGlvow2LzeF4PnCdkxUH2MCXgn9Lb7Bb1qBJTMEoPQ9Gk-l5U4VZdxqAY55qS3IgLKDh7K0Bnyg_KYE5RKOFg0hKOI4vI_0',
    },
    {
      'title': 'Oshi no Ko Season 2',
      'tier': 'Tier 2',
      'missing': 'Missing: Ep 01-11 (11 eps)',
      'spec': '[SubsPlease] 1080p AVC',
      'size': '15.4 GB',
      'status': 'Ready to queue',
      'poster': 'https://lh3.googleusercontent.com/aida-public/AB6AXuArfcK78z8aiePHzYToRBGe55HO_E_kt10lqkYjcCUV71j7Zm0e_So7UDKQvlNpjHmUDupZqrm0ehoOmpFF4cFLpQjxeJTUXULsml0dDovTFUPopXrl5bPDnD90mNozX_trc_NIXZIeDHtxkWpuQt8PPzL69QE64C3227rUP8HdoYdRhV0JyRrGVuq85D0RUlPM61Kg78YmHE7bzEyflf3GI11FLILcsqzVAUugPeQky6sgWcjG965B',
    },
  ];

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
                  _buildHeader(),

                  // 5-Step Horizontal Progress Stepper
                  _buildHorizontalStepper(),

                  // Main Scrollable Body
                  Padding(
                    padding: const EdgeInsets.all(24),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        // Bento Headline Metric Card
                        _buildHeadlineBentoCard(),
                        const SizedBox(height: 16),

                        // Daily Cap Warning Box
                        _buildDailyCapWarningBox(),
                        const SizedBox(height: 16),

                        // Expandable Ingestion Preview List
                        _buildDetectedSeriesManifest(state),
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

  Widget _buildHeader() {
    return Container(
      padding: const EdgeInsets.all(24),
      decoration: const BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 42,
            height: 42,
            decoration: BoxDecoration(
              color: ObsidianColors.secondary.withOpacity(0.2),
              borderRadius: BorderRadius.circular(10),
            ),
            child: const Icon(Icons.terminal_rounded, size: 24, color: ObsidianColors.primary),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text('GENRL DOWNLOADER SETUP WIZARD', style: ObsidianTypography.labelBadge(color: ObsidianColors.primary)),
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHighest,
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text('v2.4.1', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                    ),
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1.5),
                      decoration: BoxDecoration(
                        color: ObsidianColors.secondary.withOpacity(0.2),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text('Step 5 of 5', style: ObsidianTypography.labelBadge(color: ObsidianColors.secondary)),
                    ),
                  ],
                ),
                const SizedBox(height: 6),
                Text('Initial Library Synchronization & Review', style: ObsidianTypography.headlineMd()),
                const SizedBox(height: 4),
                Text(
                  'Review detected library manifests, ingestion queue volume, and confirm automation daemon state before starting.',
                  style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
                ),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.close_rounded, color: ObsidianColors.outline),
            onPressed: widget.onComplete,
          ),
        ],
      ),
    );
  }

  Widget _buildHorizontalStepper() {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
      color: ObsidianColors.surfaceContainerLowest,
      child: Row(
        children: [
          _buildStepItem(stepNum: 1, title: 'MAL Account', subtitle: 'ren_kurokawa', isDone: true, isActive: false),
          _buildStepDivider(isDone: true),
          _buildStepItem(stepNum: 2, title: 'qBittorrent', subtitle: ':8080 OK', isDone: true, isActive: false),
          _buildStepDivider(isDone: true),
          _buildStepItem(stepNum: 3, title: 'Root Path', subtitle: '/mnt/anime/TV', isDone: true, isActive: false),
          _buildStepDivider(isDone: true),
          _buildStepItem(stepNum: 4, title: 'Quality / Groups', subtitle: '1080p • 5 Groups', isDone: true, isActive: false),
          _buildStepDivider(isDone: false),
          _buildStepItem(stepNum: 5, title: 'Review & Launch', subtitle: 'Ready', isDone: false, isActive: true),
        ],
      ),
    );
  }

  Widget _buildStepItem({
    required int stepNum,
    required String title,
    required String subtitle,
    required bool isDone,
    required bool isActive,
  }) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: 22,
          height: 22,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: isActive
                ? ObsidianColors.primary
                : (isDone ? ObsidianColors.tertiary.withOpacity(0.2) : ObsidianColors.surfaceContainerHigh),
          ),
          child: isDone
              ? const Icon(Icons.check_rounded, size: 14, color: ObsidianColors.tertiary)
              : Center(
                  child: Text(
                    '$stepNum',
                    style: ObsidianTypography.labelBadge(
                      color: isActive ? ObsidianColors.onPrimary : ObsidianColors.outline,
                    ),
                  ),
                ),
        ),
        const SizedBox(width: 6),
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              title,
              style: ObsidianTypography.labelBadge(
                color: isActive ? ObsidianColors.primary : ObsidianColors.onSurface,
              ).copyWith(fontWeight: isActive ? FontWeight.bold : FontWeight.normal),
            ),
            Text(
              subtitle,
              style: ObsidianTypography.labelBadge(
                color: isDone ? ObsidianColors.tertiary : ObsidianColors.outline,
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildStepDivider({required bool isDone}) {
    return Expanded(
      child: Container(
        height: 1.5,
        margin: const EdgeInsets.symmetric(horizontal: 8),
        color: isDone ? ObsidianColors.tertiary.withOpacity(0.4) : ObsidianColors.primary.withOpacity(0.4),
      ),
    );
  }

  Widget _buildHeadlineBentoCard() {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    const Icon(Icons.sync_alt_rounded, size: 16, color: ObsidianColors.primary),
                    const SizedBox(width: 6),
                    Text('QUEUE PIPELINE ANALYSIS', style: ObsidianTypography.labelBadge(color: ObsidianColors.primary)),
                  ],
                ),
                const SizedBox(height: 6),
                RichText(
                  text: TextSpan(
                    style: ObsidianTypography.headlineLg(),
                    children: const [
                      TextSpan(text: 'This will queue '),
                      TextSpan(text: '214 episodes', style: TextStyle(color: ObsidianColors.primary, fontWeight: FontWeight.bold)),
                      TextSpan(text: ' across '),
                      TextSpan(text: '31 shows', style: TextStyle(color: ObsidianColors.tertiary, fontWeight: FontWeight.bold)),
                    ],
                  ),
                ),
                const SizedBox(height: 10),
                Wrap(
                  spacing: 6,
                  runSpacing: 4,
                  children: [
                    _buildPill('31 Tracked Series', ObsidianColors.surfaceContainerHigh, ObsidianColors.onSurface),
                    _buildPill('214 Missing Backlog', ObsidianColors.secondary.withOpacity(0.2), ObsidianColors.secondary),
                    _buildPill('Est. Storage ~248 GB', ObsidianColors.tertiary.withOpacity(0.2), ObsidianColors.tertiary),
                    _buildPill('Nyaa.si + Tokyo Toshokan', ObsidianColors.surfaceContainerHighest, ObsidianColors.onSurfaceVariant),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(width: 16),
          // Micro Telemetry Sparkline
          Container(
            width: 160,
            padding: const EdgeInsets.all(10),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text('Index Snatches', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                    Text('100%', style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary).copyWith(fontWeight: FontWeight.bold)),
                  ],
                ),
                const SizedBox(height: 8),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  crossAxisAlignment: CrossAxisAlignment.end,
                  children: [
                    _buildBar(10, ObsidianColors.surfaceContainerHigh),
                    _buildBar(18, ObsidianColors.primaryContainer),
                    _buildBar(26, ObsidianColors.primary),
                    _buildBar(22, ObsidianColors.primary),
                    _buildBar(30, ObsidianColors.tertiary),
                    _buildBar(14, ObsidianColors.tertiary),
                    _buildBar(25, ObsidianColors.secondary),
                    _buildBar(30, ObsidianColors.primaryContainer),
                  ],
                ),
                const SizedBox(height: 6),
                Align(
                  alignment: Alignment.centerRight,
                  child: Text('31/31 metadata resolved', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildBar(double height, Color color) {
    return Container(
      width: 10,
      height: height,
      decoration: BoxDecoration(
        color: color,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(2)),
      ),
    );
  }

  Widget _buildPill(String text, Color bg, Color textCol) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(text, style: ObsidianTypography.labelBadge(color: textCol)),
    );
  }

  Widget _buildDailyCapWarningBox() {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Icon(Icons.warning_rounded, size: 22, color: ObsidianColors.primary),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Text('Daily Ingestion Cap Exceeded Warning', style: ObsidianTypography.titleSm().copyWith(fontWeight: FontWeight.bold)),
                        Container(
                          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainer,
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Text('Cap: 30 eps/day', style: ObsidianTypography.monoCode(color: ObsidianColors.primary)),
                        ),
                      ],
                    ),
                    const SizedBox(height: 4),
                    Text(
                      'Your download cap is configured to 30 episodes/day in client preferences. Committing all 214 backlog episodes will schedule the first 30 today and throttle the remaining 184 into staged daily queues to prevent tracker rate-limits and ISP bandwidth alarms.',
                      style: ObsidianTypography.bodySm(color: ObsidianColors.onSurfaceVariant),
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 10),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
            decoration: BoxDecoration(
              color: ObsidianColors.surfaceContainerLowest.withOpacity(0.6),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Row(
                  children: [
                    Checkbox(
                      value: _bypassDailyCap,
                      onChanged: (v) => setState(() => _bypassDailyCap = v ?? false),
                      activeColor: ObsidianColors.primary,
                    ),
                    Text('Bypass daily cap for initial first-run sync ', style: ObsidianTypography.bodySm().copyWith(fontWeight: FontWeight.bold)),
                    Text('(Queue all 214 immediately without daily delay)', style: ObsidianTypography.monoCode(color: ObsidianColors.outline)),
                  ],
                ),
                Text(
                  _bypassDailyCap ? 'Full queue release' : 'Staged batching active',
                  style: ObsidianTypography.labelBadge(color: _bypassDailyCap ? ObsidianColors.warning : ObsidianColors.outline),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildDetectedSeriesManifest(StateProvider state) {
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainer,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Row(
                children: [
                  Text('Detected Series Manifest', style: ObsidianTypography.titleSm().copyWith(fontWeight: FontWeight.bold)),
                  const SizedBox(width: 8),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerHigh,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text('31 Shows Found', style: ObsidianTypography.labelBadge(color: ObsidianColors.onSurfaceVariant)),
                  ),
                ],
              ),
              Row(
                children: [
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainerLowest,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Row(
                      children: [
                        const Icon(Icons.filter_list_rounded, size: 14, color: ObsidianColors.outline),
                        const SizedBox(width: 4),
                        Text('Missing backlog only', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                      ],
                    ),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 12),

          ListView.separated(
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            itemCount: _detectedSeries.length,
            separatorBuilder: (_, __) => const SizedBox(height: 8),
            itemBuilder: (context, index) {
              final item = _detectedSeries[index];
              return Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLow,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  children: [
                    // 2:3 Cover Poster
                    Container(
                      width: 36,
                      height: 54,
                      decoration: BoxDecoration(
                        color: ObsidianColors.surfaceContainerHighest,
                        borderRadius: BorderRadius.circular(4),
                      ),
                      clipBehavior: Clip.antiAlias,
                      child: Image.network(
                        item['poster'] as String,
                        fit: BoxFit.cover,
                        errorBuilder: (_, __, ___) => const Icon(Icons.movie_rounded, size: 18, color: ObsidianColors.outline),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Text(item['title'] as String, style: ObsidianTypography.titleSm().copyWith(fontWeight: FontWeight.bold)),
                              const SizedBox(width: 6),
                              Container(
                                padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
                                decoration: BoxDecoration(
                                  color: ObsidianColors.primaryContainer,
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text(item['tier'] as String, style: ObsidianTypography.labelBadge(color: ObsidianColors.onPrimaryContainer)),
                              ),
                            ],
                          ),
                          const SizedBox(height: 4),
                          Row(
                            children: [
                              Text(item['missing'] as String, style: ObsidianTypography.labelBadge(color: ObsidianColors.tertiary)),
                              const SizedBox(width: 6),
                              Text('•', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                              const SizedBox(width: 6),
                              Text(item['spec'] as String, style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                              const SizedBox(width: 6),
                              Text('•', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                              const SizedBox(width: 6),
                              Text(item['size'] as String, style: ObsidianTypography.labelBadge(color: ObsidianColors.outline)),
                            ],
                          ),
                        ],
                      ),
                    ),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                      decoration: BoxDecoration(
                        color: ObsidianColors.secondary.withOpacity(0.2),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Row(
                        children: [
                          Container(width: 5, height: 5, decoration: const BoxDecoration(shape: BoxShape.circle, color: ObsidianColors.secondary)),
                          const SizedBox(width: 6),
                          Text('Ready to queue', style: ObsidianTypography.labelBadge(color: ObsidianColors.secondary)),
                        ],
                      ),
                    ),
                  ],
                ),
              );
            },
          ),
        ],
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
              if (_currentStep > 0) setState(() => _currentStep--);
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
