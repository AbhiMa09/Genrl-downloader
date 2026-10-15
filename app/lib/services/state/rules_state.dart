import '../../models/activity_item.dart';
import '../../models/rule_item.dart';
import 'state_context.dart';

mixin RulesStateMixin on StateContext {
  List<RuleItem> _rules = [];
  bool _isLoadingRules = false;
  String? _rulesError;

  List<RuleItem> get rules => _rules;
  RuleItem? get globalRule => _rules.where((r) => r.isGlobal).firstOrNull;
  bool get isLoadingRules => _isLoadingRules;
  String? get rulesError => _rulesError;

  @override
  Future<void> refreshRules() async {
    _isLoadingRules = true;
    _rulesError = null;
    notifyListeners();
    try {
      final globalData = await bridge.getRules();
      if (globalData != null) {
        final globalRule = RuleItem.fromJson(globalData, showTitle: 'Global Defaults');
        final overrides = _rules.where((r) => !r.isGlobal).toList();
        _rules = [globalRule, ...overrides];
      }
    } catch (e) {
      _rulesError = e.toString();
    } finally {
      _isLoadingRules = false;
      notifyListeners();
    }
  }

  Future<void> updateRule(String key, dynamic value, {int? malId}) async {
    await bridge.updateRule(key, value, malId: malId);
    await refreshRules();
    addActivityEvent(
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Updated rule "$key" for ${malId != null ? "show #$malId" : "global defaults"}',
        timestamp: DateTime.now(),
        malId: malId,
      ),
    );
    notifyListeners();
  }

  Future<void> resetRule({String? key, int? malId}) async {
    await bridge.resetRule(key: key, malId: malId);
    await refreshRules();
    addActivityEvent(
      ActivityItem(
        id: DateTime.now().millisecondsSinceEpoch,
        level: ActivityLevel.info,
        message: 'Reset rule(s) for ${malId != null ? "show #$malId" : "global defaults"}',
        timestamp: DateTime.now(),
        malId: malId,
      ),
    );
    notifyListeners();
  }
}
