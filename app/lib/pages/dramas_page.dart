import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../services/state_provider.dart';
import '../state/dramas_cubit.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/media_add_dialog.dart';
import '../widgets/poster_grid.dart';
import '../widgets/state_views.dart';
import 'series_detail_page.dart';

class DramasPage extends StatefulWidget {
  const DramasPage({super.key});

  @override
  State<DramasPage> createState() => _DramasPageState();
}

class _DramasPageState extends State<DramasPage> {
  final _filterController = TextEditingController();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      _getCubit(context).loadDramas();
    });
  }

  @override
  void dispose() {
    _filterController.dispose();
    super.dispose();
  }

  DramasCubit _getCubit(BuildContext context) {
    try {
      final cubit = Provider.of<DramasCubit?>(context, listen: false);
      if (cubit != null) return cubit;
    } catch (_) {}
    return Provider.of<StateProvider>(context, listen: false).dramasCubit;
  }

  DramasCubit _watchCubit(BuildContext context) {
    try {
      final cubit = Provider.of<DramasCubit?>(context);
      if (cubit != null) return cubit;
    } catch (_) {}
    return Provider.of<StateProvider>(context).dramasCubit;
  }

  void _openAddDramaDialog() {
    showDialog<void>(
      context: context,
      builder: (_) => const MediaAddDialog(kind: 'series'),
    );
  }

  @override
  Widget build(BuildContext context) {
    final cubit = _watchCubit(context);
    final state = cubit.state;

    if (state.selectedDrama != null) {
      return SeriesDetailPage(item: state.selectedDrama!, cubit: cubit);
    }

    final filterText = _filterController.text.trim().toLowerCase();
    final items = filterText.isEmpty
        ? state.dramas
        : state.dramas.where((d) {
            return d.title.toLowerCase().contains(filterText) ||
                (d.year?.toString().contains(filterText) ?? false);
          }).toList();

    return Column(
      children: [
        // Toolbar
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          decoration: const BoxDecoration(
            color: ObsidianColors.surfaceContainerLow,
            border: Border(
              bottom: BorderSide(color: ObsidianColors.border, width: 1),
            ),
          ),
          child: Row(
            children: [
              Expanded(
                child: SizedBox(
                  height: 36,
                  child: TextField(
                    controller: _filterController,
                    onChanged: (_) => setState(() {}),
                    style: ObsidianTypography.bodyMd(),
                    decoration: InputDecoration(
                      hintText: 'Filter dramas & series...',
                      hintStyle: ObsidianTypography.bodySm(
                        color: ObsidianColors.outline,
                      ),
                      prefixIcon: const Icon(
                        Icons.search,
                        size: 18,
                        color: ObsidianColors.outline,
                      ),
                      suffixIcon: _filterController.text.isNotEmpty
                          ? IconButton(
                              tooltip: 'Clear filter',
                              icon: const Icon(Icons.clear, size: 16),
                              onPressed: () {
                                _filterController.clear();
                                setState(() {});
                              },
                            )
                          : null,
                      filled: true,
                      fillColor: ObsidianColors.surfaceContainer,
                      contentPadding: const EdgeInsets.symmetric(
                        horizontal: 10,
                        vertical: 0,
                      ),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(8),
                        borderSide: const BorderSide(
                          color: ObsidianColors.border,
                        ),
                      ),
                      enabledBorder: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(8),
                        borderSide: const BorderSide(
                          color: ObsidianColors.border,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 12),
              IconButton(
                icon: const Icon(Icons.refresh, size: 20),
                tooltip: 'Refresh dramas',
                onPressed: state.isLoading ? null : () => cubit.loadDramas(),
              ),
              const SizedBox(width: 8),
              Tooltip(
                message: 'Add new drama or series from TMDB',
                child: ElevatedButton.icon(
                  onPressed: _openAddDramaDialog,
                  icon: const Icon(Icons.add, size: 18),
                  label: const Text('Add drama'),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: ObsidianColors.primary,
                    foregroundColor: ObsidianColors.onPrimary,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),

        // Body
        Expanded(
          child: Builder(
            builder: (context) {
              if (state.isLoading && state.dramas.isEmpty) {
                return const Center(
                  child: LoadingStateView(
                    message: 'Loading dramas from library...',
                  ),
                );
              }
              if (state.error != null && state.dramas.isEmpty) {
                return Center(
                  child: ErrorStateView(
                    title: 'Failed to load dramas',
                    message: state.error!,
                    onRetry: () => cubit.loadDramas(),
                    retryLabel: 'Retry',
                  ),
                );
              }
              if (state.dramas.isEmpty) {
                return Center(
                  child: EmptyStateView(
                    icon: Icons.tv_outlined,
                    title: 'No dramas yet',
                    description: 'Search and add dramas or series from TMDB to track seasons and episodes automatically.',
                    action: ElevatedButton.icon(
                      onPressed: _openAddDramaDialog,
                      icon: const Icon(Icons.add),
                      label: const Text('Add drama'),
                    ),
                  ),
                );
              }
              if (items.isEmpty) {
                return Center(
                  child: Text(
                    'No dramas matching "${_filterController.text}"',
                    style: ObsidianTypography.bodyMd(
                      color: ObsidianColors.outline,
                    ),
                  ),
                );
              }
              return PosterGrid(
                items: items,
                onItemTap: (item) => cubit.openDramaDetail(item),
              );
            },
          ),
        ),
      ],
    );
  }
}
