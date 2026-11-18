import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/main.dart';

void main() {
  group('--page argument parsing', () {
    test('integers 0-9 behave as before', () {
      expect(parsePageIndex('0'), equals(0));
      expect(parsePageIndex('1'), equals(1));
      expect(parsePageIndex('2'), equals(2));
      expect(parsePageIndex('3'), equals(3));
      expect(parsePageIndex('4'), equals(4));
      expect(parsePageIndex('5'), equals(5));
      expect(parsePageIndex('6'), equals(6));
      expect(parsePageIndex('7'), equals(7));
      expect(parsePageIndex('8'), equals(8));
      expect(parsePageIndex('9'), equals(9));
    });

    test('names map correctly to their page indices', () {
      expect(parsePageIndex('dashboard'), equals(0));
      expect(parsePageIndex('inbox'), equals(1));
      expect(parsePageIndex('queue'), equals(2));
      expect(parsePageIndex('library'), equals(3));
      expect(parsePageIndex('search'), equals(4));
      expect(parsePageIndex('activity'), equals(5));
      expect(parsePageIndex('rules'), equals(6));
      expect(parsePageIndex('settings'), equals(7));
      expect(parsePageIndex('movies'), equals(8));
      expect(parsePageIndex('dramas'), equals(9));
    });

    test('names are case-insensitive and trimmed', () {
      expect(parsePageIndex('Dashboard'), equals(0));
      expect(parsePageIndex('INBOX'), equals(1));
      expect(parsePageIndex(' Queue '), equals(2));
      expect(parsePageIndex('  library  '), equals(3));
      expect(parsePageIndex('SEARCH'), equals(4));
      expect(parsePageIndex('Activity'), equals(5));
      expect(parsePageIndex('RULES'), equals(6));
      expect(parsePageIndex('Settings'), equals(7));
      expect(parsePageIndex(' Movies '), equals(8));
      expect(parsePageIndex('DRAMAS'), equals(9));
    });

    test('unknown names fall back to 0 exactly as before', () {
      expect(parsePageIndex('unknown'), equals(0));
      expect(parsePageIndex('invalid'), equals(0));
      expect(parsePageIndex('foo_bar'), equals(0));
      expect(parsePageIndex(''), equals(0));
      expect(parsePageIndex('   '), equals(0));
      expect(parsePageIndex('999xyz'), equals(0));
    });
  });
}
