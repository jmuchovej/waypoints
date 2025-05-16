import 'package:flutter/material.dart';

// Empty implementations that do nothing in release mode
Future<void> initDevEnvironment() async {}

void addDevTools(List<Widget> screens) {}

NavigationDestination getDevToolsDestination(BuildContext context) {
  return NavigationDestination(
    icon: Icon(Icons.home_outlined, color: Colors.grey.shade400),
    selectedIcon: Icon(
      Icons.home_rounded,
      color: Theme.of(context).primaryColor,
    ),
    label: 'Tasks',
  );
}
