// bin/generate_icon_font.dart
// Dart script to generate icon fonts from SVG files

import 'dart:convert';
import 'dart:io';
import 'package:args/args.dart';
import 'package:path/path.dart' as path;
import 'package:icon_font_generator/icon_font_generator.dart';

Future<void> main(List<String> arguments) async {
  // Set up command line argument parsing
  final parser = ArgParser()
    ..addOption('icon-set-name', help: 'Display name of the icon set')
    ..addOption('prefix', help: 'Icon set prefix (used for URLs)')
    ..addFlag('help',
        abbr: 'h', help: 'Show usage information', negatable: false);

  // try {
  final results = parser.parse(arguments);

  if (results['help'] == true) {
    print('Icon Font Generator');
    print('');
    print('Usage: dart gen/icon-font.dart [options]');
    print('');
    print('Options:');
    print(parser.usage);
    return;
  }

  final iconSetName = results['icon-set-name'] as String?;
  final prefix = results['prefix'] as String?;

  if (iconSetName == null || prefix == null) {
    print('Error: Missing required arguments');
    print('');
    print('Usage: dart gen/icon-font.dart [options]');
    print('');
    print('Options:');
    print(parser.usage);
    return;
  }

  await generateIconFont(iconSetName: iconSetName, prefix: prefix);
  // } catch (e) {
  //   print('Error: $e');
  //   print('');
  //   print('Usage: dart bin/generate_icon_font.dart [options]');
  //   print('');
  //   print('Options:');
  //   print(parser.usage);
  //   return;
  // }
}

Future<void> generateIconFont({
  required String iconSetName,
  required String prefix,
}) async {
  final fontName = "Iconify $iconSetName";
  final className = iconSetName;
  final fontFilename = "$prefix.otf";
  final dartFilename = "$prefix.dart";
  final fontPath = path.join(".", "lib", "fonts", fontFilename);
  final dartPath = path.join(".", "lib", dartFilename);

  print('Generating icon font: `$fontName`...');

  // Check if the index.json exists
  final fontJson = File(path.join('icons', '$prefix.json'));
  if (!await fontJson.exists()) {
    throw Exception('Index file not found at ${fontJson.path}');
  }

  // Load the icon metadata
  final Map<String, String> svgMap =
      Map.castFrom(json.decode(await fontJson.readAsString()).map((k, v) {
    k = k.toString();
    k = k.startsWith(RegExp("\d")) ? "_$k" : k;
    k = k.replaceAll("-", "_");
    v = v.toString();
    return MapEntry(k, v);
  }));

  print('Processing ${svgMap.length} icons...');

  // Generate the font
  final otf = svgToOtf(svgMap: svgMap, fontName: fontName);
  writeToFile(fontPath, otf.font);

  final genClass = generateFlutterClass(
    glyphList: otf.glyphList,
    familyName: otf.font.familyName,
    className: className,
    fontFileName: fontFilename,
    indent: 2,
  );
  File(dartPath).writeAsStringSync(genClass, flush: true);

  print('Successfully generated font and class files:');
  print(' - Font file: $fontPath');
  print(' - Dart file: $dartPath');
  print(' - Metadata: ${fontJson.path}');
}
