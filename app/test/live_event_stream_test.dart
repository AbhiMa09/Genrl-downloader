import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/models/queue_item.dart';
import 'package:genrl_downloader/models/system_status.dart';
import 'package:genrl_downloader/services/state_provider.dart';

import 'fake_core_bridge.dart';

class StreamingCoreBridge extends FakeCoreBridge {
  int telemetryPollCount = 0;
  int currentActiveTorrents = 1;

  StreamingCoreBridge() : super(mode: FakeMode.data);

  @override
  Future<SystemStatus?> getSystemStatus() async {
    telemetryPollCount++;
    return SystemStatus(
      daemonConnected: true,
      qbitConnected: true,
      qbitVersion: 'v4.6.5',
      downloadSpeedBytes: 10485760.0 * telemetryPollCount,
      uploadSpeedBytes: 1048576,
      freeDiskSpaceGb: 400,
      totalDiskSpaceGb: 1000,
      activeTorrents: currentActiveTorrents,
      totalShows: 12,
      totalEpisodes: 140,
      automationOn: true,
      inboxQuestions: 0,
    );
  }

  @override
  Future<List<QueueItem>> getQueue() async {
    if (currentActiveTorrents > 1) {
      return const [
        QueueItem(
          id: 1,
          malId: 52991,
          showTitle: 'Sousou no Frieren',
          season: 1,
          episode: 28,
          releaseName: '[SubsPlease] Sousou no Frieren - 28.mkv',
          group: 'SubsPlease',
          progress: 0.85,
          speedBytes: 20971520,
          seeders: 210,
          leechers: 15,
          sizeBytes: 1468006400,
          etaSeconds: 12,
          state: QueueState.downloading,
        ),
        QueueItem(
          id: 2,
          malId: 54959,
          showTitle: 'Dungeon Meshi',
          season: 1,
          episode: 24,
          releaseName: '[Erai-raws] Dungeon Meshi - 24.mkv',
          group: 'Erai-raws',
          progress: 0.15,
          speedBytes: 10485760,
          seeders: 95,
          leechers: 8,
          sizeBytes: 1258291200,
          etaSeconds: 110,
          state: QueueState.downloading,
        ),
      ];
    }
    return const [
      QueueItem(
        id: 1,
        malId: 52991,
        showTitle: 'Sousou no Frieren',
        season: 1,
        episode: 28,
        releaseName: '[SubsPlease] Sousou no Frieren - 28.mkv',
        group: 'SubsPlease',
        progress: 0.50,
        speedBytes: 10485760,
        seeders: 180,
        leechers: 10,
        sizeBytes: 1468006400,
        etaSeconds: 45,
        state: QueueState.downloading,
      ),
    ];
  }
}

void main() {
  test(
    'Event stream / background timer updates state store without user action',
    () async {
      final streamBridge = StreamingCoreBridge();
      final state = StateProvider(bridge: streamBridge, startPolling: false);

      // Initial load
      await state.refreshAll();
      expect(state.status.activeTorrents, equals(1));
      expect(state.queue.length, equals(1));
      expect(state.queue.first.progress, equals(0.50));

      // Core emits an event / background state change without any user action
      streamBridge.currentActiveTorrents = 2;

      // Simulate periodic live telemetry tick
      await state.refreshLiveTelemetry();

      // Verify store updated reactively without user intervention
      expect(state.status.activeTorrents, equals(2));
      expect(state.queue.length, equals(2));
      expect(state.queue.first.progress, equals(0.85));
      expect(streamBridge.telemetryPollCount, greaterThanOrEqualTo(2));

      state.dispose();
    },
  );
}
