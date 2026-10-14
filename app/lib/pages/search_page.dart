import 'package:flutter/material.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';

class SearchPage extends StatefulWidget {
  const SearchPage({super.key});

  @override
  State<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends State<SearchPage> {
  final _searchController = TextEditingController(text: '[SubsPlease] Sousou no Frieren - 28 (1080p) [F028E44A].mkv');
  String _selectedRes = '1080p';
  String _selectedView = 'results';
  bool _trustedOnly = true;
  bool _isSearching = false;

  final List<Map<String, dynamic>> _mockResults = [
    {
      'isBest': true,
      'title': '[SubsPlease] Sousou no Frieren - 28 (1080p) [F028E44A].mkv',
      'hash': 'F028E44A',
      'indexer': 'Nyaa.si #1823901',
      'group': 'SubsPlease',
      'format': '1080p',
      'codec': 'x264 • AAC',
      'size': '1.42 GB',
      'seeds': 142,
      'leech': 18,
      'age': '2h ago',
      'matchScore': '98% Match',
      'matchTier': 'Tier 1 Target',
      'bonuses': ['+30 Tier 1 Group', '+20 1080p Match', '+15 Swarm Peak'],
    },
    {
      'isBest': false,
      'title': '[Erai-raws] Sousou no Frieren - 28 [1080p][HEVC][Multiple Subtitle].mkv',
      'hash': '8CA109BD',
      'indexer': 'Nyaa.si #1823904',
      'group': 'Erai-raws',
      'format': '1080p',
      'codec': 'x265 • Opus',
      'size': '892 MB',
      'seeds': 89,
      'leech': 12,
      'age': '3h ago',
      'matchScore': '92% Match',
      'matchTier': 'Tier 2 Valid',
      'bonuses': ['+20 HEVC Tag', '+10 Preferred Track'],
    },
    {
      'isBest': false,
      'title': '[Judas] Sousou no Frieren - 28 [1080p][HEVC x265 10bit][Dual-Audio].mkv',
      'hash': '44AB21C0',
      'indexer': 'Nyaa.si #1823922',
      'group': 'Judas',
      'format': '1080p',
      'codec': 'HEVC • Dual-Audio',
      'size': '850 MB',
      'seeds': 52,
      'leech': 8,
      'age': '4h ago',
      'matchScore': '88% Match',
      'matchTier': 'Small Size',
      'bonuses': ['+10 Dual Audio', '-5 Secondary Release'],
    },
    {
      'isBest': false,
      'title': '[Beatrice-Raws] Sousou no Frieren - 28 [BDRip 1920x1080 HEVC FLAC]',
      'hash': '19DA77E1',
      'indexer': 'Nyaa.si #1823955',
      'group': 'Beatrice-Raws',
      'format': '1080p BD',
      'codec': 'HEVC • FLAC 2.0',
      'size': '3.40 GB',
      'seeds': 34,
      'leech': 2,
      'age': '5h ago',
      'matchScore': '85% Match',
      'matchTier': 'Archival Remux',
      'bonuses': ['+25 Lossless Audio', '-10 Large Size'],
    },
  ];

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  void _triggerSearch() {
    setState(() => _isSearching = true);
    Future.delayed(const Duration(milliseconds: 600), () {
      if (mounted) setState(() => _isSearching = false);
    });
  }

  void _grabRelease(String title) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text('Queued grab for: $title'),
        backgroundColor: ObsidianColors.success,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // 1. Header Section with Indexer Status Chips
          _buildHeader(),
          const SizedBox(height: 12),

          // 2. Quick Preset Queries
          _buildPresets(),
          const SizedBox(height: 16),

          // 3. Search Bar & Target Assignment (Top Card)
          _buildSearchCard(),
          const SizedBox(height: 16),

          // 4. Granular Filter Toolbar
          _buildFilterToolbar(),
          const SizedBox(height: 16),

          // 5. View Mode Switcher & Results Summary
          _buildViewSwitcher(),
          const SizedBox(height: 12),

          // 6. Compact Dense Results Table View
          if (_selectedView == 'zero')
            _buildZeroState()
          else if (_selectedView == 'skeleton')
            _buildSkeletonState()
          else
            _buildResultsTable(),
        ],
      ),
    );
  }

  Widget _buildHeader() {
    return Wrap(
      alignment: WrapAlignment.spaceBetween,
      crossAxisAlignment: WrapCrossAlignment.center,
      spacing: 16,
      runSpacing: 10,
      children: [
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text('Manual Indexer Search', style: ObsidianTypography.headlineLg()),
                const SizedBox(width: 8),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                    color: ObsidianColors.primary.withOpacity(0.12),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text('V2.4 CORE', style: ObsidianTypography.labelBadge(color: ObsidianColors.primary, fontSize: 10)),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Text(
              'Direct query via Prowlarr / Nyaa.si API with automatic rule scoring and hash inspection',
              style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
            ),
          ],
        ),

        // Indexer Status Chips
        Wrap(
          spacing: 8,
          runSpacing: 6,
          children: [
            _buildIndexerChip('Nyaa.si', 'Online • 42ms', ObsidianColors.tertiary),
            _buildIndexerChip('Tokyo Toshokan', 'Idle', ObsidianColors.outline),
            _buildIndexerChip('Prowlarr Bridge', 'Connected', ObsidianColors.primary),
          ],
        ),
      ],
    );
  }

  Widget _buildPresets() {
    final presets = [
      'Frieren S01',
      'Dungeon Meshi Complete',
      'Oshi no Ko S2 1080p',
      'Kimi no Na wa Remux',
      'Bleach TYBW Ep 21',
    ];

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: Row(
        children: [
          Row(
            children: [
              const Icon(Icons.history_rounded, size: 14, color: ObsidianColors.outline),
              const SizedBox(width: 4),
              Text('PRESETS: ', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline, fontSize: 9)),
            ],
          ),
          const SizedBox(width: 6),
          ...presets.map((p) {
            return Padding(
              padding: const EdgeInsets.only(right: 6.0),
              child: InkWell(
                onTap: () {
                  _searchController.text = p;
                  _triggerSearch();
                },
                borderRadius: BorderRadius.circular(6),
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerHigh,
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(color: ObsidianColors.border, width: 0.5),
                  ),
                  child: Text(p, style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface, fontSize: 10)),
                ),
              ),
            );
          }),
        ],
      ),
    );
  }

  Widget _buildSearchCard() {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          LayoutBuilder(
            builder: (context, constraints) {
              final isStacked = constraints.maxWidth < 800;

              return Flex(
                direction: isStacked ? Axis.vertical : Axis.horizontal,
                children: [
                  // Main Query Input
                  Expanded(
                    flex: isStacked ? 0 : 8,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            Text('QUERY STRING / TORZNAB QUERY', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline, fontSize: 9)),
                            Text('Auto-Sanitizing CRC32 hashes', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 9)),
                          ],
                        ),
                        const SizedBox(height: 6),
                        Container(
                          height: 44,
                          padding: const EdgeInsets.symmetric(horizontal: 10),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainerLowest,
                            borderRadius: BorderRadius.circular(8),
                            border: Border.all(color: ObsidianColors.border),
                          ),
                          child: Row(
                            children: [
                              const Icon(Icons.search_rounded, size: 20, color: ObsidianColors.outline),
                              const SizedBox(width: 8),
                              Expanded(
                                child: TextField(
                                  controller: _searchController,
                                  style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface, fontSize: 12),
                                  decoration: InputDecoration(
                                    hintText: 'Search anime release or torrent hash...',
                                    hintStyle: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 12),
                                    border: InputBorder.none,
                                    isDense: true,
                                  ),
                                  onSubmitted: (_) => _triggerSearch(),
                                ),
                              ),
                              IconButton(
                                icon: const Icon(Icons.close_rounded, size: 16, color: ObsidianColors.outline),
                                onPressed: () => _searchController.clear(),
                                padding: EdgeInsets.zero,
                                constraints: const BoxConstraints(),
                              ),
                              const SizedBox(width: 6),
                              Container(
                                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                                decoration: BoxDecoration(
                                  color: ObsidianColors.surfaceContainerHighest,
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                child: Text('ENTER', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 9)),
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                  if (!isStacked) const SizedBox(width: 14) else const SizedBox(height: 14),

                  // Target Binding Dropdown / Preview
                  Expanded(
                    flex: isStacked ? 0 : 4,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            Text('LIBRARY TARGET BINDING', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline, fontSize: 9)),
                            Text('Linked', style: ObsidianTypography.monoCode(color: ObsidianColors.tertiary, fontSize: 9)),
                          ],
                        ),
                        const SizedBox(height: 6),
                        Container(
                          height: 44,
                          padding: const EdgeInsets.symmetric(horizontal: 8),
                          decoration: BoxDecoration(
                            color: ObsidianColors.surfaceContainerLowest,
                            borderRadius: BorderRadius.circular(8),
                            border: Border.all(color: ObsidianColors.border),
                          ),
                          child: Row(
                            children: [
                              Container(
                                width: 28,
                                height: 32,
                                decoration: BoxDecoration(
                                  color: ObsidianColors.surfaceContainerHighest,
                                  borderRadius: BorderRadius.circular(4),
                                ),
                                clipBehavior: Clip.antiAlias,
                                child: Image.network(
                                  'https://cdn.myanimelist.net/images/anime/1015/138006.jpg',
                                  fit: BoxFit.cover,
                                  errorBuilder: (_, __, ___) => const Icon(Icons.movie_rounded, size: 14, color: ObsidianColors.outline),
                                ),
                              ),
                              const SizedBox(width: 8),
                              Expanded(
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  mainAxisAlignment: MainAxisAlignment.center,
                                  children: [
                                    Text('Sousou no Frieren', style: ObsidianTypography.titleSm(fontSize: 11), maxLines: 1, overflow: TextOverflow.ellipsis),
                                    Text('S01E28 • Final Arc', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 9)),
                                  ],
                                ),
                              ),
                              const Icon(Icons.unfold_more_rounded, size: 16, color: ObsidianColors.outline),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              );
            },
          ),
          const SizedBox(height: 12),

          // Action Button Strip
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Row(
                children: [
                  Container(width: 6, height: 6, decoration: const BoxDecoration(color: ObsidianColors.primary, shape: BoxShape.circle)),
                  const SizedBox(width: 6),
                  Text('Auto-categorized as: Anime / 1080p Web-DL', style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10)),
                ],
              ),
              Row(
                children: [
                  OutlinedButton.icon(
                    onPressed: () => _searchController.clear(),
                    icon: const Icon(Icons.restart_alt_rounded, size: 14),
                    label: Text('Reset Query', style: ObsidianTypography.bodySm(fontSize: 11)),
                    style: OutlinedButton.styleFrom(
                      side: const BorderSide(color: ObsidianColors.border),
                      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                    ),
                  ),
                  const SizedBox(width: 8),
                  ElevatedButton.icon(
                    onPressed: _isSearching ? null : _triggerSearch,
                    icon: _isSearching
                        ? const SizedBox(width: 14, height: 14, child: CircularProgressIndicator(strokeWidth: 2, color: ObsidianColors.onPrimary))
                        : const Icon(Icons.travel_explore_rounded, size: 16),
                    label: Text('Search Indexers', style: ObsidianTypography.titleSm(color: ObsidianColors.onPrimary, fontSize: 11)),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: ObsidianColors.primary,
                      foregroundColor: ObsidianColors.onPrimary,
                      elevation: 0,
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildFilterToolbar() {
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
              Text('RES: ', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline, fontSize: 9)),
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
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(color: ObsidianColors.surfaceContainer, borderRadius: BorderRadius.circular(6)),
                child: Row(
                  children: [
                    Text('GROUP: ', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline, fontSize: 8)),
                    Text('SubsPlease (Preferred)', style: ObsidianTypography.monoCode(color: ObsidianColors.primary, fontSize: 10)),
                  ],
                ),
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(color: ObsidianColors.surfaceContainer, borderRadius: BorderRadius.circular(6)),
                child: Row(
                  children: [
                    Text('SWARM: ', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline, fontSize: 8)),
                    Text('Seeds ≥ 10', style: ObsidianTypography.monoMetric(color: ObsidianColors.tertiary, fontSize: 10)),
                  ],
                ),
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(color: ObsidianColors.surfaceContainer, borderRadius: BorderRadius.circular(6)),
                child: Text('800 MB — 2.5 GB', style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface, fontSize: 10)),
              ),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Checkbox(
                    value: _trustedOnly,
                    onChanged: (val) => setState(() => _trustedOnly = val ?? true),
                    activeColor: ObsidianColors.primary,
                  ),
                  Text('Trusted / VIP Only', style: ObsidianTypography.bodySm(fontSize: 11)),
                ],
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildResPill(String res) {
    final isSelected = _selectedRes == res;
    return InkWell(
      onTap: () => setState(() => _selectedRes = res),
      borderRadius: BorderRadius.circular(4),
      child: Container(
        margin: const EdgeInsets.only(right: 4),
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          color: isSelected ? ObsidianColors.primary : ObsidianColors.surfaceContainer,
          borderRadius: BorderRadius.circular(4),
        ),
        child: Text(
          res,
          style: ObsidianTypography.monoCode(
            color: isSelected ? ObsidianColors.onPrimary : ObsidianColors.outline,
            fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
            fontSize: 10,
          ),
        ),
      ),
    );
  }

  Widget _buildViewSwitcher() {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Row(
          children: [
            Text('Found 14 releases across 2 indexers', style: ObsidianTypography.titleSm(fontSize: 12)),
            const SizedBox(width: 6),
            Container(width: 6, height: 6, decoration: const BoxDecoration(color: ObsidianColors.tertiary, shape: BoxShape.circle)),
          ],
        ),
        Row(
          children: [
            _buildViewTab('results', 'Results (14)'),
            const SizedBox(width: 4),
            _buildViewTab('skeleton', 'Skeleton View'),
            const SizedBox(width: 4),
            _buildViewTab('zero', 'Zero State View'),
          ],
        ),
      ],
    );
  }

  Widget _buildViewTab(String key, String label) {
    final isSelected = _selectedView == key;
    return InkWell(
      onTap: () => setState(() => _selectedView = key),
      borderRadius: BorderRadius.circular(6),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          color: isSelected ? ObsidianColors.surfaceContainerHigh : Colors.transparent,
          borderRadius: BorderRadius.circular(6),
        ),
        child: Text(
          label,
          style: ObsidianTypography.monoCode(
            color: isSelected ? ObsidianColors.primary : ObsidianColors.outline,
            fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
            fontSize: 10,
          ),
        ),
      ),
    );
  }

  Widget _buildResultsTable() {
    return Container(
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      clipBehavior: Clip.antiAlias,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Table Heading
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
            color: ObsidianColors.surfaceContainerHigh,
            child: Row(
              children: [
                Expanded(flex: 6, child: Text('TORRENT / RELEASE SPECIFICATION', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                Expanded(flex: 2, child: Text('GROUP', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                Expanded(flex: 2, child: Text('FORMAT', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                Expanded(flex: 2, child: Text('SIZE', textAlign: TextAlign.right, style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                Expanded(flex: 2, child: Text('SWARM (S/L)', textAlign: TextAlign.center, style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                Expanded(flex: 1, child: Text('AGE', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                Expanded(flex: 2, child: Text('SCORE', style: ObsidianTypography.labelBadge(color: ObsidianColors.outline))),
                const SizedBox(width: 80, child: Text('ACTION', textAlign: TextAlign.right, style: TextStyle(fontSize: 10, color: ObsidianColors.outline))),
              ],
            ),
          ),

          // Table Rows
          ListView.separated(
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            itemCount: _mockResults.length,
            separatorBuilder: (_, __) => const Divider(height: 1, color: ObsidianColors.border),
            itemBuilder: (context, index) {
              final r = _mockResults[index];
              final isBest = r['isBest'] as bool;

              return Container(
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                decoration: BoxDecoration(
                  color: isBest ? ObsidianColors.surfaceContainer.withOpacity(0.8) : Colors.transparent,
                  border: isBest ? Border.all(color: ObsidianColors.primary.withOpacity(0.4)) : null,
                ),
                child: Row(
                  children: [
                    // 1. Torrent Spec & Bonuses
                    Expanded(
                      flex: 6,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              if (isBest) ...[
                                Container(
                                  padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1.5),
                                  decoration: BoxDecoration(color: ObsidianColors.primary, borderRadius: BorderRadius.circular(4)),
                                  child: Row(
                                    mainAxisSize: MainAxisSize.min,
                                    children: [
                                      const Icon(Icons.star_rounded, size: 10, color: ObsidianColors.onPrimary),
                                      const SizedBox(width: 3),
                                      Text('Best Rule Match', style: ObsidianTypography.labelBadge(color: ObsidianColors.onPrimary, fontSize: 8)),
                                    ],
                                  ),
                                ),
                                const SizedBox(width: 6),
                              ],
                              Expanded(
                                child: Text(
                                  r['title'] as String,
                                  style: ObsidianTypography.monoCode(
                                    color: ObsidianColors.onSurface,
                                    fontWeight: isBest ? FontWeight.bold : FontWeight.normal,
                                    fontSize: 11,
                                  ),
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 4),
                          Wrap(
                            spacing: 8,
                            children: [
                              Text(r['hash'] as String, style: ObsidianTypography.monoCode(color: ObsidianColors.primary, fontSize: 9)),
                              ...(r['bonuses'] as List<String>).map((b) => Text(b, style: ObsidianTypography.monoCode(color: ObsidianColors.tertiary, fontSize: 9))),
                              Text(r['indexer'] as String, style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 9)),
                            ],
                          ),
                        ],
                      ),
                    ),

                    // 2. Group
                    Expanded(
                      flex: 2,
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                        decoration: BoxDecoration(color: ObsidianColors.surfaceContainerHighest, borderRadius: BorderRadius.circular(4)),
                        child: Text(r['group'] as String, style: ObsidianTypography.monoCode(color: ObsidianColors.primary, fontSize: 10), maxLines: 1, overflow: TextOverflow.ellipsis),
                      ),
                    ),

                    // 3. Format
                    Expanded(
                      flex: 2,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(r['format'] as String, style: ObsidianTypography.monoMetric(color: ObsidianColors.onSurface, fontWeight: FontWeight.bold, fontSize: 10)),
                          Text(r['codec'] as String, style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 9)),
                        ],
                      ),
                    ),

                    // 4. Size
                    Expanded(
                      flex: 2,
                      child: Text(r['size'] as String, textAlign: TextAlign.right, style: ObsidianTypography.monoMetric(color: ObsidianColors.onSurface, fontSize: 10)),
                    ),

                    // 5. Swarm
                    Expanded(
                      flex: 2,
                      child: Text(
                        '${r['seeds']} / ${r['leech']}',
                        textAlign: TextAlign.center,
                        style: ObsidianTypography.monoMetric(color: ObsidianColors.tertiary, fontWeight: FontWeight.bold, fontSize: 10),
                      ),
                    ),

                    // 6. Age
                    Expanded(
                      flex: 1,
                      child: Text(r['age'] as String, style: ObsidianTypography.monoCode(color: ObsidianColors.outline, fontSize: 10)),
                    ),

                    // 7. Score
                    Expanded(
                      flex: 2,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(r['matchScore'] as String, style: ObsidianTypography.monoMetric(color: isBest ? ObsidianColors.primary : ObsidianColors.onSurface, fontWeight: FontWeight.bold, fontSize: 10)),
                          Text(r['matchTier'] as String, style: ObsidianTypography.labelBadge(color: ObsidianColors.outline, fontSize: 8)),
                        ],
                      ),
                    ),

                    // 8. Grab / Queue Button
                    SizedBox(
                      width: 80,
                      child: ElevatedButton.icon(
                        onPressed: () => _grabRelease(r['title'] as String),
                        icon: const Icon(Icons.download_rounded, size: 12),
                        label: Text(isBest ? 'Grab' : 'Queue', style: ObsidianTypography.titleSm(color: isBest ? ObsidianColors.onPrimary : ObsidianColors.onSurface, fontSize: 10)),
                        style: ElevatedButton.styleFrom(
                          backgroundColor: isBest ? ObsidianColors.primary : ObsidianColors.surfaceContainer,
                          foregroundColor: isBest ? ObsidianColors.onPrimary : ObsidianColors.onSurface,
                          elevation: 0,
                          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
                        ),
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

  Widget _buildZeroState() {
    return Container(
      padding: const EdgeInsets.all(32),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: ObsidianColors.border),
      ),
      child: Column(
        children: [
          const Icon(Icons.travel_explore_rounded, size: 48, color: ObsidianColors.outline),
          const SizedBox(height: 12),
          Text('No Torrent Results Found', style: ObsidianTypography.headlineMd()),
          const SizedBox(height: 6),
          Text(
            'Try widening your size filter range, enabling non-trusted uploaders, or querying Tokyo Toshokan.',
            style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }

  Widget _buildSkeletonState() {
    return Column(
      children: List.generate(4, (index) {
        return Container(
          margin: const EdgeInsets.only(bottom: 8),
          height: 52,
          decoration: BoxDecoration(
            color: ObsidianColors.surfaceContainerLow,
            borderRadius: BorderRadius.circular(8),
          ),
        );
      }),
    );
  }

  Widget _buildIndexerChip(String name, String status, Color dotColor) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: ObsidianColors.surfaceContainerLow,
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: ObsidianColors.border, width: 0.5),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(width: 6, height: 6, decoration: BoxDecoration(color: dotColor, shape: BoxShape.circle)),
          const SizedBox(width: 6),
          Text(name, style: ObsidianTypography.monoCode(color: ObsidianColors.onSurface, fontSize: 10)),
          const SizedBox(width: 4),
          Text(status, style: ObsidianTypography.monoCode(color: dotColor, fontSize: 9)),
        ],
      ),
    );
  }
}
