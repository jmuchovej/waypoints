import 'package:flutter/material.dart';
import 'package:flux/dev/database/seed.dart';
import 'package:flux/dev/screens/devtools.dart';

// This function will be called in debug mode only
Future<void> initDevEnvironment() async {
  await SeedDataService.seedIfNeeded();
}

// Adds the dev tools screen to the list of screens
void addDevTools(List<Widget> screens) {
  screens.add(DevToolsScreen());
}

// Returns the navigation destination for dev tools
NavigationDestination getDevToolsDestination(BuildContext context) {
  return const NavigationDestination(
    icon: Icon(Icons.developer_mode_outlined),
    selectedIcon: Icon(Icons.developer_mode),
    label: 'Dev Tools',
  );
}
