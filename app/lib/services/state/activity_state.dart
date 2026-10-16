import '../../models/activity_item.dart';
import 'state_context.dart';

mixin ActivityStateMixin on StateContext {
  List<ActivityItem> _events = [];
  bool _isLoadingEvents = false;
  String? _eventsError;

  List<ActivityItem> get events => _events;
  bool get isLoadingEvents => _isLoadingEvents;
  String? get eventsError => _eventsError;

  @override
  void addActivityEvent(ActivityItem item) {
    _events.insert(0, item);
    notifyListeners();
  }

  Future<void> checkMetadataUpdates(List<ActivityItem> items) async {
    final currentShow = selectedShow;
    if (currentShow != null && currentShow.metadataStatus != 'ready') {
      final hasMetadataUpdate = items.any(
        (e) =>
            (e.malId == currentShow.malId ||
                e.message.contains('#${currentShow.malId}') ||
                e.message.toLowerCase().contains(
                  currentShow.title.toLowerCase(),
                )) &&
            (e.message.toLowerCase().contains('metadata') ||
                e.message.toLowerCase().contains('synopsis')),
      );
      if (hasMetadataUpdate) {
        await loadShowDetail(currentShow.malId);
      }
    }
  }

  Future<void> setEventsFromTelemetry(List<ActivityItem> items) async {
    _events = items;
    _eventsError = null;
    await checkMetadataUpdates(items);
  }

  @override
  Future<void> refreshEvents() async {
    _isLoadingEvents = true;
    _eventsError = null;
    try {
      final items = await bridge.getEvents();
      _events = items;
      await checkMetadataUpdates(items);
    } catch (e) {
      _eventsError = e.toString();
    } finally {
      _isLoadingEvents = false;
      notifyListeners();
    }
  }
}
