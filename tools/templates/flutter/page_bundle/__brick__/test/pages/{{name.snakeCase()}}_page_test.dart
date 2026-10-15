import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/pages/{{name.snakeCase()}}/{{name.snakeCase()}}_page.dart';

void main() {
  testWidgets('renders {{name.pascalCase()}}Page without errors', (WidgetTester tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: {{name.pascalCase()}}Page(),
      ),
    );

    // Initial pump (loading state)
    expect(find.byType(CircularProgressIndicator), findsOneWidget);

    // Advance timer past async loader
    await tester.pumpAndSettle();

    expect(find.text('{{name.pascalCase()}}'), findsOneWidget);
    expect(find.text('Item 1'), findsOneWidget);
  });
}
