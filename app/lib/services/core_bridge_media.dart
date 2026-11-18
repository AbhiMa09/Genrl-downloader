import '../models/media_item.dart';
import 'core_bridge.dart';
import 'core_bridge_media_parsers.dart';
import 'core_bridge_result.dart';

export 'core_bridge_media_parsers.dart';

/// `media search|add|list|remove` bindings for the movies and dramas pages.
extension CoreBridgeMedia on CoreBridge {
  /// TMDB search. The CLI answers `{"error":"tmdb_not_configured"}` with exit
  /// 0 when no key is stored, which surfaces as [MediaSearchOutcome.error].
  Future<MediaSearchOutcome> searchMedia(
    String query, {
    String kind = 'movie',
    int? year,
  }) async {
    final args = <String>['media', 'search', query, '--kind', kind, '--json'];
    if (year != null) args.addAll(['--year', year.toString()]);
    final res = await execute(args);
    final out = stdoutOf(res);
    final cliError = parseCliError(out);
    if (cliError != null) return MediaSearchOutcome(error: cliError);
    if (res.exitCode != 0) {
      return MediaSearchOutcome(error: cliFailure(res));
    }
    return MediaSearchOutcome(items: parseMediaSearch(out));
  }

  /// Adds (or refreshes) a title by TMDB id and queues its missing episodes.
  Future<MediaAddOutcome> addMedia(int tmdbId, {String kind = 'movie'}) async {
    final res = await execute([
      'media',
      'add',
      tmdbId.toString(),
      '--kind',
      kind,
      '--json',
    ]);
    final out = stdoutOf(res);
    final cliError = parseCliError(out);
    if (cliError != null) return MediaAddOutcome(error: cliError);
    if (res.exitCode != 0) return MediaAddOutcome(error: cliFailure(res));
    final result = parseMediaAdd(out);
    if (result == null) {
      return const MediaAddOutcome(
        error: CoreCommandError(
          'bad_response',
          'Unreadable media add response',
        ),
      );
    }
    return MediaAddOutcome(result: result);
  }

  /// Managed titles for one kind (`movie` or `series`).
  Future<List<MediaListItem>> listMedia({String? kind}) async {
    final args = <String>['media', 'list'];
    if (kind != null && kind.isNotEmpty) args.addAll(['--kind', kind]);
    args.add('--json');
    final res = await execute(args);
    if (res.exitCode != 0) return const [];
    return parseMediaList(stdoutOf(res));
  }

  /// Removes a title, its episodes and any active jobs.
  Future<bool> removeMedia(int mediaId) async {
    final res = await execute([
      'media',
      'remove',
      mediaId.toString(),
      '--json',
    ]);
    if (res.exitCode != 0) return false;
    final obj = tryJsonObject(stdoutOf(res));
    return obj?['ok'] == true;
  }
}
