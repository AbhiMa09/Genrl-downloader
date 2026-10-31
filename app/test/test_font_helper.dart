import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:google_fonts/google_fonts.dart';

Future<void> loadAppFonts() async {
  TestWidgetsFlutterBinding.ensureInitialized();
  GoogleFonts.config.allowRuntimeFetching = false;

  Future<ByteData> readFont(String path) async {
    File file = File(path);
    if (!file.existsSync()) {
      file = File('app/$path');
    }
    if (!file.existsSync()) {
      throw Exception('Font file not found: $path (also checked app/$path)');
    }
    final bytes = await file.readAsBytes();
    return ByteData.sublistView(Uint8List.fromList(bytes));
  }

  // 1. Inter
  final interReg = await readFont('assets/fonts/Inter-Regular.ttf');
  final interMed = await readFont('assets/fonts/Inter-Medium.ttf');
  final interSemi = await readFont('assets/fonts/Inter-SemiBold.ttf');
  final interBold = await readFont('assets/fonts/Inter-Bold.ttf');

  for (final family in [
    'Roboto',
    'Inter',
    'Inter_regular',
    'Inter_medium',
    'Inter_semiBold',
    'Inter_bold',
    'Inter_400',
    'Inter_500',
    'Inter_600',
    'Inter_700',
    'packages/google_fonts/Inter',
    'packages/google_fonts/Inter_regular',
    'packages/google_fonts/Inter_medium',
    'packages/google_fonts/Inter_semiBold',
    'packages/google_fonts/Inter_bold',
    'packages/google_fonts/Inter_400',
    'packages/google_fonts/Inter_500',
    'packages/google_fonts/Inter_600',
    'packages/google_fonts/Inter_700',
  ]) {
    final loader = FontLoader(family);
    loader.addFont(Future.value(interReg));
    loader.addFont(Future.value(interMed));
    loader.addFont(Future.value(interSemi));
    loader.addFont(Future.value(interBold));
    await loader.load();
  }

  // 2. JetBrains Mono
  final jbReg = await readFont('assets/fonts/JetBrainsMono-Regular.ttf');
  final jbMed = await readFont('assets/fonts/JetBrainsMono-Medium.ttf');
  final jbSemi = await readFont('assets/fonts/JetBrainsMono-SemiBold.ttf');
  final jbBold = await readFont('assets/fonts/JetBrainsMono-Bold.ttf');

  for (final family in [
    'JetBrainsMono',
    'JetBrains Mono',
    'JetBrainsMono_regular',
    'JetBrainsMono_medium',
    'JetBrainsMono_semiBold',
    'JetBrainsMono_bold',
    'JetBrainsMono_400',
    'JetBrainsMono_500',
    'JetBrainsMono_600',
    'JetBrainsMono_700',
    'packages/google_fonts/JetBrainsMono',
    'packages/google_fonts/JetBrains Mono',
    'packages/google_fonts/JetBrainsMono_regular',
    'packages/google_fonts/JetBrainsMono_medium',
    'packages/google_fonts/JetBrainsMono_semiBold',
    'packages/google_fonts/JetBrainsMono_bold',
    'packages/google_fonts/JetBrainsMono_400',
    'packages/google_fonts/JetBrainsMono_500',
    'packages/google_fonts/JetBrainsMono_600',
    'packages/google_fonts/JetBrainsMono_700',
  ]) {
    final loader = FontLoader(family);
    loader.addFont(Future.value(jbReg));
    loader.addFont(Future.value(jbMed));
    loader.addFont(Future.value(jbSemi));
    loader.addFont(Future.value(jbBold));
    await loader.load();
  }

  // 3. Material Icons
  final matIcons = await readFont('assets/fonts/MaterialIcons-Regular.otf');
  final matLoader = FontLoader('MaterialIcons');
  matLoader.addFont(Future.value(matIcons));
  await matLoader.load();

  // 4. Material Symbols
  final symOutlined = await readFont(
    'assets/fonts/MaterialSymbolsOutlined.ttf',
  );
  final symRounded = await readFont('assets/fonts/MaterialSymbolsRounded.ttf');

  for (final family in [
    'MaterialSymbols',
    'Material Symbols',
    'Material Symbols Rounded',
    'Material Symbols Outlined',
  ]) {
    final loader = FontLoader(family);
    loader.addFont(Future.value(symOutlined));
    loader.addFont(Future.value(symRounded));
    await loader.load();
  }
}
