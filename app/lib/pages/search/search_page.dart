import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../services/state_provider.dart';
import '../../theme/colors.dart';
import '../../theme/typography.dart';
import 'search_card.dart';
import 'search_filter_toolbar.dart';
import 'search_header.dart';
import 'search_results_table.dart';

class SearchPage extends StatefulWidget {
  const SearchPage({super.key});

  @override
  State<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends State<SearchPage> {
  final _searchController = TextEditingController();
  String _selectedRes = '1080p';
  String _selectedView = 'results';
  bool _trustedOnly = true;
  bool _isSearching = false;

  List<Map<String, dynamic>> _searchResults = [];

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  Future<void> _triggerSearch() async {
    final query = _searchController.text.trim();
    if (query.isEmpty) {
      return;
    }
    setState(() => _isSearching = true);
    try {
      final state = context.read<StateProvider>();
      final items = await state.bridge.search(query);
      if (mounted) {
        setState(() {
          _searchResults = items.asMap().entries.map((entry) {
            final idx = entry.key;
            final it = entry.value;
            final parsed = (it['parsed'] as Map<String, dynamic>?) ?? {};
            return {
              'isBest': idx == 0,
              'title': it['title'] ?? 'Unknown Torrent',
              'hash': (it['info_hash'] ?? '--------').toString().substring(
                  0,
                  8.clamp(
                      0, (it['info_hash'] ?? '').toString().length)),
              'indexer': 'Nyaa.si',
              'group': parsed['group'] ?? '-',
              'format': parsed['resolution'] ?? '1080p',
              'codec': parsed['codec'] ?? '-',
              'size': it['size_text'] ?? '-',
              'seeds': it['seeders'] ?? 0,
              'leech': it['leechers'] ?? 0,
              'age': '-',
              'matchScore': '${it['seeders'] ?? 0} seeds',
              'matchTier':
                  (it['trusted'] == true) ? 'Trusted' : 'Standard',
              'bonuses': (it['trusted'] == true)
                  ? ['+30 Trusted Uploader']
                  : <String>[],
              'magnet': it['magnet'],
              'link': it['link'],
            };
          }).toList();
        });
      }
    } catch (_) {
    } finally {
      if (mounted) {
        setState(() => _isSearching = false);
      }
    }
  }

  void _grabRelease(String title) {
    final item = _searchResults.firstWhere((r) => r['title'] == title,
        orElse: () => {});
    final target = (item['magnet'] as String?)?.isNotEmpty == true
        ? item['magnet'] as String
        : ((item['link'] as String?)?.isNotEmpty == true
            ? item['link'] as String
            : title);
    try {
      context.read<StateProvider>().bridge.grab(target);
    } catch (_) {}
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text('Queued grab for: $title'),
        backgroundColor: ObsidianColors.success,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // 1. Header Section with Indexer Status Chips & Presets
          SearchHeader(
            state: state,
            onSelectPreset: (preset) {
              _searchController.text = preset;
              _triggerSearch();
            },
          ),
          const SizedBox(height: 16),

          // 2. Search Bar & Target Assignment (Top Card)
          SearchCard(
            state: state,
            searchController: _searchController,
            isSearching: _isSearching,
            onSearch: _triggerSearch,
            onReset: () => _searchController.clear(),
          ),
          const SizedBox(height: 16),

          // 3. Granular Filter Toolbar
          SearchFilterToolbar(
            selectedRes: _selectedRes,
            onSelectRes: (res) => setState(() => _selectedRes = res),
            trustedOnly: _trustedOnly,
            onToggleTrusted: (val) => setState(() => _trustedOnly = val),
          ),
          const SizedBox(height: 16),

          // 4. View Mode Switcher & Results Summary
          _buildViewSwitcher(),
          const SizedBox(height: 12),

          // 5. Compact Dense Results Table View
          if (_isSearching)
            _buildSkeletonState()
          else if (_selectedView == 'zero' || _searchResults.isEmpty)
            _buildZeroState()
          else if (_selectedView == 'skeleton')
            _buildSkeletonState()
          else
            SearchResultsTable(
              searchResults: _searchResults,
              onGrabRelease: _grabRelease,
            ),
        ],
      ),
    );
  }

  Widget _buildViewSwitcher() {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Row(
          children: [
            Text('Found ${_searchResults.length} releases',
                style: ObsidianTypography.titleSm(fontSize: 12)),
            const SizedBox(width: 6),
            Container(
                width: 6,
                height: 6,
                decoration: const BoxDecoration(
                    color: ObsidianColors.tertiary, shape: BoxShape.circle)),
          ],
        ),
        Row(
          children: [
            _buildViewTab('results', 'Results (${_searchResults.length})'),
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
          color: isSelected
              ? ObsidianColors.surfaceContainerHigh
              : Colors.transparent,
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
          const Icon(Icons.travel_explore_rounded,
              size: 48, color: ObsidianColors.outline),
          const SizedBox(height: 12),
          Text('No Torrent Results Found',
              style: ObsidianTypography.headlineMd()),
          const SizedBox(height: 6),
          Text(
            'Try widening your size filter range, enabling non-trusted uploaders, or searching for an alternative title.',
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
}
