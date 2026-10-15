import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../services/state_provider.dart';
import '../../widgets/state_views.dart';
import 'dashboard_airing_posters.dart';
import 'dashboard_bottom_grid.dart';
import 'dashboard_health_pulse_bar.dart';
import 'dashboard_kpi_metrics.dart';
import 'dashboard_operational_bar.dart';
import 'dashboard_pipeline_table.dart';

class DashboardPage extends StatefulWidget {
  const DashboardPage({super.key});

  @override
  State<DashboardPage> createState() => _DashboardPageState();
}

class _DashboardPageState extends State<DashboardPage> {
  final _searchFilterController = TextEditingController();

  @override
  void dispose() {
    _searchFilterController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    if (state.isLoadingStatus && state.queue.isEmpty && state.shows.isEmpty && state.coreError == null) {
      return const Center(
        child: LoadingStateView(message: 'Querying system telemetry from genrl core...'),
      );
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (state.coreError != null) ...[
            ErrorStateView(
              title: 'Core Engine Unreachable',
              message: state.coreError!,
              onRetry: () => state.refreshAll(),
              retryLabel: 'Retry Connection',
            ),
            const SizedBox(height: 16),
          ],

          // 1. Operational Command & Action Bar
          DashboardOperationalBar(
            state: state,
            searchFilterController: _searchFilterController,
          ),
          const SizedBox(height: 12),

          // 2. Health & State Pulse Bar
          DashboardHealthPulseBar(state: state),
          const SizedBox(height: 16),

          // 3. KPI Metric Strip (4 Cards)
          DashboardKpiMetrics(state: state),
          const SizedBox(height: 20),

          // 4. Daemon Pipeline & Active Ingestion Table
          DashboardPipelineTable(state: state),
          const SizedBox(height: 24),

          // 5. Airing Today & Monitored Next Up (2:3 Poster Grid)
          DashboardAiringPosters(state: state),
          const SizedBox(height: 24),

          // 6. Bottom Split Grid: Ingestions Ledger + Tracker Matrix
          DashboardBottomGrid(state: state),
          const SizedBox(height: 16),
        ],
      ),
    );
  }
}
