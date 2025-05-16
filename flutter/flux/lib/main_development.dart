import 'package:flutter/material.dart';
import 'package:flux/config/environment.dart';
import 'package:flux/dev/database/seed.dart';
import 'package:flux/dev/screens/devtools.dart';
import 'package:flux/objectbox.dart';
import 'package:flux/app.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  EnvironmentConfig.setEnvironment(Environment.development);

  await ObjectBox.create();
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

  runApp(TempoFlux(routes: routes));
}
