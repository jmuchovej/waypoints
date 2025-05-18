import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flux/dev/database/seed.dart';
import 'package:flux/dev/screens/devtools.dart';
import 'package:flux/objectbox.dart';
import 'package:flux/app.dart';

Future<void> preRunApp() async {
  // debugPaintSizeEnabled = true;
  // Optional: Enable other debug flags
  // debugPaintBaselinesEnabled = true; // Shows text baselines
  // debugPaintPointersEnabled = true; // Shows pointer hit testing
  // debugPaintLayerBordersEnabled = true; // Shows layer boundaries

  await SeedDataService.seedIfNeeded();
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
        icon: Icon(Icons.developer_mode_outlined, color: Colors.grey.shade400),
        selectedIcon: Icon(
          Icons.developer_mode,
          color: Theme.of(context).primaryColor,
        ),
        label: 'Dev Tools',
      ),
    ),
  ];

  runApp(ProviderScope(child: TempoFlux(routes: routes)));
}
