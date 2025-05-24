import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:tempo/infrastructure/_logging.dart';
import 'package:tempo/dev/database/seed.dart';
import 'package:tempo/dev/screens/devtools.dart';
import 'package:tempo/objectbox.dart';
import 'package:tempo/app.dart';

Future<void> preRunApp() async {
  // debugPaintSizeEnabled = true;
  // Optional: Enable other debug flags
  // debugPaintBaselinesEnabled = true; // Shows text baselines
  debugPaintLayerBordersEnabled = true;
  // debugPaintPointersEnabled = true; // Shows pointer hit testing
  // debugPaintLayerBordersEnabled = true; // Shows layer boundaries

  await SeedDataService.seedIfNeeded();

  initLoggers(Level.FINEST, tempoLoggers);
  initLoggers(Level.FINER, superEditorLoggers);
  deactivateLoggers(FTLLoggers);
}

List<AppRoute> appRoutes() {
  return [
    ...defaultAppRoutes,
    AppRoute(
      DevToolsScreen(),
      (context) => NavigationDestination(
        icon: Icon(Icons.developer_mode_outlined, color: Colors.grey.shade400),
        selectedIcon: Icon(
          Icons.developer_mode,
          color: Theme.of(context).primaryColor,
        ),
        label: 'Dev Tools',
      ),
    ),
  ];
}

Future<void> main() async {
  // debugPaintSizeEnabled = true;
  // Optional: Enable other debug flags
  debugPaintBaselinesEnabled = true; // Shows text baselines
  // debugPaintPointersEnabled = true; // Shows pointer hit testing
  debugPaintLayerBordersEnabled = true; // Shows layer boundaries

  obx = await ObjectBox.create();
  await SeedDataService.seedIfNeeded();

  final routes = [
    ...defaultAppRoutes,
    AppRoute(
      DevToolsScreen(),
      (context) => NavigationDestination(
        icon: Icon(Icons.developer_mode_outlined, color: ColorScheme.of(context).primary),
        selectedIcon: Icon(
          Icons.developer_mode,
          color: ColorScheme.of(context).primary,
        ),
        label: 'Dev Tools',
      ),
    ),
  ];

  runApp(ProviderScope(child: TempoFlux(routes: routes)));
}
