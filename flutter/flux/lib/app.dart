import 'package:flutter/material.dart';
import 'package:flux/flavors.dart';
import 'package:flux/objectbox.dart';
import 'package:flux/pages/home.dart';
import 'package:flux/pages/projects.dart';
import 'package:flux/pages/super_task.dart';
import 'package:flux/pages/tasks.dart';

late ObjectBox obx;

class AppRoute {
  final Widget screen;
  final NavigationDestination Function(BuildContext) destination;

  AppRoute(this.screen, this.destination);
}

final defaultAppRoutes = [
  AppRoute(
    TasksScreen(),
    (context) => NavigationDestination(
      icon: Icon(Icons.home_outlined, color: Colors.grey.shade400),
      selectedIcon: Icon(
        Icons.home_rounded,
        color: Theme.of(context).primaryColor,
      ),
      label: 'Tasks',
    ),
  ),
  AppRoute(
    ProjectsScreen(),
    (context) => NavigationDestination(
      icon: Icon(Icons.folder_outlined, color: Colors.grey.shade400),
      selectedIcon: Icon(
        Icons.folder_rounded,
        color: Theme.of(context).primaryColor,
      ),
      label: 'Projects',
    ),
  ),
  AppRoute(
    SuperTaskScreen(),
    (context) => NavigationDestination(
      icon: Icon(Icons.power_outlined, color: Colors.grey.shade400),
      selectedIcon: Icon(Icons.power, color: Theme.of(context).primaryColor),
      label: 'SuperTask',
    ),
  ),
];

class TempoFlux extends StatelessWidget {
  final List<Widget> screens;
  final List<NavigationDestination Function(BuildContext)> destinations;

  TempoFlux({super.key, required List<AppRoute> routes})
    : screens = routes.map((route) => route.screen).toList(),
      destinations = routes.map((route) => route.destination).toList();

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: F.title,
      theme: ThemeData(
        primarySwatch: Colors.blue,
        visualDensity: VisualDensity.adaptivePlatformDensity,
        useMaterial3: true,
      ),
      home: HomeScreen(screens: screens, destinations: destinations),
    );
  }
}
