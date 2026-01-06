import 'package:flutter/material.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import "package:waypoints/src/generated/rust/frb_generated.dart";
import 'package:widgetbook/widgetbook.dart';
import 'package:widgetbook_annotation/widgetbook_annotation.dart' as widgetbook;

import 'addons/brightness_addon.dart';
import "main.directories.g.dart";

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const ProviderScope(child: WidgetbookApp()));
}

@widgetbook.App()
class WidgetbookApp extends StatelessWidget {
  const WidgetbookApp({super.key});

  @override
  Widget build(BuildContext context) {
    final widgetbook = Widgetbook.material(
      // The [directories] variable does not exist yet,
      // it will be generated in the next step
      directories: directories,
      addons: [
        ViewportAddon(Viewports.all),
        AlignmentAddon(),
        InspectorAddon(),
        // LocalizationAddon(),
        TextScaleAddon(),
        ZoomAddon(),
        BrightnessAddon(),
      ],
      appBuilder: (context, child) {
        return child;
      },
    );
    return widgetbook;
  }
}
