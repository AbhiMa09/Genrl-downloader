import 'package:flutter/material.dart';
import '../../models/activity_item.dart';
import '../../models/library_item.dart';
import '../core_bridge.dart';

abstract class StateContext extends ChangeNotifier {
  CoreBridge get bridge;

  void addActivityEvent(ActivityItem item);

  Future<void> refreshStatus();
  Future<void> refreshQueue();
  Future<void> refreshShows({String? status});
  Future<void> refreshInbox();
  Future<void> refreshEvents();
  Future<void> refreshRules();
  Future<void> refreshMalStatus();
  Future<void> refreshLocations();
  Future<void> loadShowDetail(int malId);
  Future<void> forceSyncMal();

  void selectPage(int index);

  List<LibraryShow> get shows;
  LibraryShow? get selectedShow;
  set selectedShow(LibraryShow? value);

  Future<void> downloadEpisode(int malId, int episodeNumber);
}
