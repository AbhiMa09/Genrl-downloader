import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../models/media_item.dart';
import '../services/state_provider.dart';
import '../state/dramas_cubit.dart';
import '../theme/colors.dart';
import '../theme/typography.dart';
import '../widgets/cyber_badge.dart';
import '../widgets/media_add_dialog.dart';
import '../widgets/poster_grid.dart';
import '../widgets/state_views.dart';

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

  void _showDramaDetail(MediaListItem item, DramasCubit cubit) {
    showDialog<void>(
      context: context,
      builder: (dialogCtx) => _DramaDetailDialog(item: item, cubit: cubit),
    );
  }

  @override
  Widget build(BuildContext context) {
    final cubit = _watchCubit(context);
    final state = cubit.state;

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
              ElevatedButton.icon(
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
                onItemTap: (item) => _showDramaDetail(item, cubit),
              );
            },
          ),
        ),
      ],
    );
  }
}

class _DramaDetailDialog extends StatelessWidget {
  final MediaListItem item;
  final DramasCubit cubit;

  const _DramaDetailDialog({required this.item, required this.cubit});

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      backgroundColor: ObsidianColors.surfaceContainerHigh,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      titlePadding: const EdgeInsets.fromLTRB(24, 20, 16, 12),
      contentPadding: const EdgeInsets.symmetric(horizontal: 24, vertical: 8),
      actionsAlignment: MainAxisAlignment.spaceBetween,
      actionsPadding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
      title: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Expanded(
            child: Text(
              item.year != null ? '${item.title} (${item.year})' : item.title,
              style: ObsidianTypography.titleSm(fontSize: 16),
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          IconButton(
            icon: const Icon(Icons.close, size: 20),
            onPressed: () => Navigator.of(context).pop(),
          ),
        ],
      ),
      content: SizedBox(
        width: 480,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (item.posterUrl != null)
                  ClipRRect(
                    borderRadius: BorderRadius.circular(8),
                    child: SizedBox(
                      width: 90,
                      height: 135,
                      child: Image.network(
                        item.posterUrl!,
                        fit: BoxFit.cover,
                        cacheWidth: 180,
                        errorBuilder: (_, __, ___) => Container(
                          color: ObsidianColors.surfaceContainer,
                          child: const Icon(
                            Icons.tv_outlined,
                            color: ObsidianColors.outline,
                          ),
                        ),
                      ),
                    ),
                  )
                else
                  Container(
                    width: 90,
                    height: 135,
                    decoration: BoxDecoration(
                      color: ObsidianColors.surfaceContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: const Icon(
                      Icons.tv_outlined,
                      size: 32,
                      color: ObsidianColors.outline,
                    ),
                  ),
                const SizedBox(width: 16),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Text('Status: ', style: ObsidianTypography.bodySm()),
                          CyberBadge.fromStatus(item.status),
                        ],
                      ),
                      const SizedBox(height: 8),
                      if (item.progress != null)
                        Row(
                          children: [
                            Text(
                              'Progress: ',
                              style: ObsidianTypography.bodySm(),
                            ),
                            Text(
                              item.progress!.formatSummary(),
                              style: ObsidianTypography.monoCode(
                                color: ObsidianColors.primary,
                              ),
                            ),
                          ],
                        ),
                      const SizedBox(height: 8),
                      if (item.tmdbId != null)
                        Text(
                          'TMDB ID: ${item.tmdbId}',
                          style: ObsidianTypography.monoCode(
                            color: ObsidianColors.outline,
                          ),
                        ),
                      const SizedBox(height: 8),
                      if (item.overview != null && item.overview!.isNotEmpty)
                        Text(
                          item.overview!,
                          style: ObsidianTypography.bodySm(
                            color: ObsidianColors.onSurfaceVariant,
                          ),
                          maxLines: 4,
                          overflow: TextOverflow.ellipsis,
                        ),
                    ],
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
      actions: [
        TextButton.icon(
          icon: const Icon(
            Icons.delete_outline,
            size: 18,
            color: ObsidianColors.error,
          ),
          label: const Text(
            'Remove',
            style: TextStyle(color: ObsidianColors.error),
          ),
          onPressed: () async {
            final nav = Navigator.of(context);
            await cubit.removeDrama(item.id);
            if (nav.mounted) {
              nav.pop();
            }
          },
        ),
        if (item.status == 'failed' && item.tmdbId != null)
          ElevatedButton.icon(
            icon: const Icon(Icons.refresh, size: 18),
            label: const Text('Retry'),
            style: ElevatedButton.styleFrom(
              backgroundColor: ObsidianColors.surfaceContainer,
              foregroundColor: ObsidianColors.primary,
            ),
            onPressed: () async {
              final nav = Navigator.of(context);
              await cubit.addDrama(item.tmdbId!);
              if (nav.mounted) {
                nav.pop();
              }
            },
          ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Close'),
        ),
      ],
    );
  }
}
