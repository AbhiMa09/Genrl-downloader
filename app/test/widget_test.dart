import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:genrl_downloader/main.dart';

class _MockHttpOverrides extends HttpOverrides {
  @override
  HttpClient createHttpClient(SecurityContext? context) {
    return super.createHttpClient(context);
  }
}

void main() {
  testWidgets('Genrl Downloader desktop app shell smoke test', (WidgetTester tester) async {
    // Set desktop screen resolution
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(() => tester.view.resetPhysicalSize());

    await tester.pumpWidget(const GenrlApp());
    await tester.pump();

    // Verify main navigation rail and header are rendered
    expect(find.text('Genrl'), findsOneWidget);
    expect(find.text('Dashboard Overview'), findsOneWidget);
    expect(find.text('ACTIVE DOWNLOAD TRANSFERS'), findsOneWidget);
  });
}
