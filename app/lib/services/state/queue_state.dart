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

  /// Jobs still in the pipeline: not completed and not failed.
  int get pendingQueueCount => _queue
      .where(
        (i) => i.state != QueueState.completed && i.state != QueueState.failed,
      )
      .length;

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

  Future<bool> recheckJob(int jobId) async {
    final ok = await bridge.stepJob(jobId);
    await refreshQueue();
    await refreshStatus();
    return ok;
  }

  Future<bool> cancelJob(int jobId, {bool deleteFiles = false}) async {
    final ok = await bridge.cancelJob(jobId, deleteFiles: deleteFiles);
    removeQueueItem(jobId);
    await refreshQueue();
    await refreshStatus();
    return ok;
  }

  Future<bool> retryJob(int jobId) async {
    final ok = await bridge.retryJob(jobId);
    await refreshQueue();
    await refreshStatus();
    return ok;
  }

  Future<bool> retryAllJobs() async {
    final ok = await bridge.retryAllJobs();
    await refreshQueue();
    await refreshStatus();
    return ok;
  }
}
