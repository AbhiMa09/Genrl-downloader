import '../../models/queue_item.dart';
import 'state_context.dart';

mixin QueueStateMixin on StateContext {
  List<QueueItem> _queue = [];
  bool _isLoadingQueue = false;
  String? _queueError;

  List<QueueItem> get queue => _queue;
  bool get isLoadingQueue => _isLoadingQueue;
  String? get queueError => _queueError;

  List<QueueItem> get activeDownloads =>
      _queue.where((q) => q.state == QueueState.downloading).toList();

  int get activeQueueCount =>
      _queue.where((i) => i.state == QueueState.downloading).length;

  void setQueueFromTelemetry(List<QueueItem> items) {
    _queue = items;
    _queueError = null;
  }

  @override
  Future<void> refreshQueue() async {
    _isLoadingQueue = true;
    _queueError = null;
    try {
      final items = await bridge.getQueue();
      _queue = items;
    } catch (e) {
      _queueError = e.toString();
    } finally {
      _isLoadingQueue = false;
      notifyListeners();
    }
  }

  void removeQueueItem(int id) {
    _queue.removeWhere((item) => item.id == id);
    notifyListeners();
  }
}
