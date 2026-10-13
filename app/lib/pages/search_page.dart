import 'package:flutter/material.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';

class SearchPage extends StatefulWidget {
  const SearchPage({super.key});

  @override
  State<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends State<SearchPage> {
  final _searchController = TextEditingController();
  bool _isLoading = false;
  List<Map<String, dynamic>> _results = [];

  void _performSearch() {
    final query = _searchController.text.trim();
    if (query.isEmpty) return;

    setState(() => _isLoading = true);
    // Simulate / fetch Nyaa results
    Future.delayed(const Duration(milliseconds: 600), () {
      if (!mounted) return;
      setState(() {
        _isLoading = false;
        _results = [
          {
            'title': '[SubsPlease] $query - 01 (1080p) [A1B2C3D4].mkv',
            'group': 'SubsPlease',
            'resolution': '1080p',
            'seeders': 68,
            'leechers': 3,
            'size': '1.34 GB',
            'trusted': true,
          },
          {
            'title': '[Erai-raws] $query - 01 [1080p][MULTi].mkv',
            'group': 'Erai-raws',
            'resolution': '1080p',
            'seeders': 45,
            'leechers': 2,
            'size': '1.41 GB',
            'trusted': true,
          },
          {
            'title': '[Judas] $query - S01E01 [BD1080p][HEVC x265].mkv',
            'group': 'Judas',
            'resolution': '1080p BD',
            'seeders': 32,
            'leechers': 1,
            'size': '850 MB',
            'trusted': true,
          },
        ];
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text('Search Nyaa Releases', style: ObsidianTypography.headlineLg()),
          const SizedBox(height: 6),
          Text(
            'Direct release finder routed through the limit-proof Gate token bucket.',
            style: ObsidianTypography.bodySm(color: ObsidianColors.outline),
          ),
          const SizedBox(height: 20),

          // Search input bar
          Row(
            children: [
              Expanded(
                child: Container(
                  height: 42,
                  decoration: BoxDecoration(
                    color: ObsidianColors.surfaceContainerLowest,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: ObsidianColors.border),
                  ),
                  padding: const EdgeInsets.symmetric(horizontal: 14),
                  alignment: Alignment.centerLeft,
                  child: TextField(
                    controller: _searchController,
                    onSubmitted: (_) => _performSearch(),
                    style: ObsidianTypography.bodyMd(color: ObsidianColors.onSurface),
                    decoration: InputDecoration(
                      hintText: 'Enter anime title or release query...',
                      hintStyle: ObsidianTypography.bodyMd(color: ObsidianColors.outline),
                      border: InputBorder.none,
                      isDense: true,
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 12),
              ElevatedButton.icon(
                onPressed: _isLoading ? null : _performSearch,
                icon: const Icon(Icons.search_rounded, size: 18),
                label: const Text('Search'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: ObsidianColors.primaryContainer,
                  foregroundColor: ObsidianColors.onPrimaryContainer,
                  elevation: 0,
                  padding: const EdgeInsets.symmetric(
                      horizontal: 18, vertical: 12),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 24),

          // Search results
          if (_isLoading)
            const Center(
              child: Padding(
                padding: EdgeInsets.all(32),
                child: CircularProgressIndicator(
                  valueColor: AlwaysStoppedAnimation<Color>(ObsidianColors.primary),
                ),
              ),
            )
          else if (_results.isEmpty)
            Container(
              padding: const EdgeInsets.all(40),
              decoration: BoxDecoration(
                color: ObsidianColors.surfaceContainerLow,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: ObsidianColors.border),
              ),
              child: Center(
                child: Text(
                  'No search results yet. Search for a title above to discover torrent candidates.',
                  style: ObsidianTypography.bodyMd(color: ObsidianColors.outline),
                ),
              ),
            )
          else
            ..._results.map((r) {
              return Container(
                margin: const EdgeInsets.only(bottom: 12),
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: ObsidianColors.surfaceContainerLow,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: ObsidianColors.border),
                ),
                child: Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(r['title'] as String,
                              style: ObsidianTypography.titleSm()),
                          const SizedBox(height: 6),
                          Row(
                            children: [
                              Text('Group: ${r['group']}',
                                  style: ObsidianTypography.monoCode(
                                      color: ObsidianColors.primary)),
                              const SizedBox(width: 12),
                              Text('Size: ${r['size']}',
                                  style: ObsidianTypography.monoCode(
                                      color: ObsidianColors.onSurfaceVariant)),
                              const SizedBox(width: 12),
                              Text('Seeds: ${r['seeders']}',
                                  style: ObsidianTypography.monoCode(
                                      color: ObsidianColors.tertiary)),
                            ],
                          ),
                        ],
                      ),
                    ),
                    ElevatedButton.icon(
                      onPressed: () {
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(
                            backgroundColor: ObsidianColors.surfaceContainerHigh,
                            content: Text(
                              'Queued release for download: ${r['group']}',
                              style: ObsidianTypography.bodySm(color: ObsidianColors.primary),
                            ),
                          ),
                        );
                      },
                      icon: const Icon(Icons.download_rounded, size: 16),
                      label: const Text('Grab'),
                      style: ElevatedButton.styleFrom(
                        backgroundColor: ObsidianColors.surfaceContainerHigh,
                        foregroundColor: ObsidianColors.primary,
                        elevation: 0,
                        side: const BorderSide(color: ObsidianColors.border),
                      ),
                    ),
                  ],
                ),
              );
            }),
        ],
      ),
    );
  }
}
