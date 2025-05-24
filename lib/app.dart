import 'package:flutter/material.dart';
import 'package:tempo/flavors.dart';
import 'package:tempo/objectbox.dart';
import 'package:tempo/pages/home.dart';
import 'package:tempo/pages/projects.dart';
import 'package:tempo/modules/flux/pages/tasks.dart';

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
      icon: Icon(
        Icons.format_list_bulleted_rounded,
        color: ColorScheme.of(context).primaryContainer,
      ),
      selectedIcon: Icon(
        Icons.format_list_bulleted_rounded,
        color: ColorScheme.of(context).primary,
      ),
      label: 'Tasks',
    ),
  ),
  AppRoute(
    ProjectsScreen(),
    (context) => NavigationDestination(
      icon: Icon(Icons.folder_outlined, color: ColorScheme.of(context).primaryContainer),
      selectedIcon: Icon(
        Icons.folder_rounded,
        color: ColorScheme.of(context).primary,
      ),
      label: 'Projects',
    ),
  ),
];

final lightTheme = ThemeData(
  colorScheme: ColorScheme.fromSeed(
    seedColor: Colors.blue,
    brightness: Brightness.light,
  ),
  useMaterial3: true,
);

final darkTheme = ThemeData(
  colorScheme: ColorScheme.fromSeed(
    seedColor: Colors.blue,
    brightness: Brightness.dark,
  ),
  useMaterial3: true,
);

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
      theme: lightTheme,
      darkTheme: darkTheme,
      themeMode: ThemeMode.system,
      home: HomeScreen(screens: screens, destinations: destinations),
      debugShowCheckedModeBanner: false,
      // debugShowMaterialGrid: true,
      // initialRoute: "/",
    );
  }
}
