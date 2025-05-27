import 'package:flutter/material.dart';
import 'package:tempo/flavors.dart';
import 'package:tempo/src/database/objectbox.dart';
import 'package:tempo/pages/projects.dart';
import 'package:tempo/flux/pages/tasks.dart';
import 'package:tempo/theme/theme.dart';

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

// final lightTheme = ThemeData(
//   colorScheme: ColorScheme.fromSeed(
//     seedColor: Colors.blue,
//     brightness: Brightness.light,
//   ),
//   useMaterial3: true,
// );
//
// final darkTheme = ThemeData(
//   colorScheme: ColorScheme.fromSeed(
//     seedColor: Colors.blue,
//     brightness: Brightness.dark,
//   ),
//   useMaterial3: true,
// );

class Tempo extends StatelessWidget {
  Tempo({super.key, required List<AppRoute> routes})
    : screens = routes.map((route) => route.screen).toList(),
      destinations = routes.map((route) => route.destination).toList();

  final List<Widget> screens;
  final List<NavigationDestination Function(BuildContext)> destinations;

  @override
  Widget build(BuildContext context) {
    TextTheme textTheme = createTextTheme(context, display: "Lato", body: "Lato");
    TempoTheme theme = TempoTheme(textTheme);

    return MaterialApp(
      title: F.title,
      theme: theme.light(),
      darkTheme: theme.dark(),
      themeMode: ThemeMode.system,
      home: HomeScreen(screens: screens, destinations: destinations),
      debugShowCheckedModeBanner: false,
      // debugShowMaterialGrid: true,
      // initialRoute: "/",
    );
  }
}

class HomeScreen extends StatefulWidget {
  final List<Widget> screens;
  final List<NavigationDestination Function(BuildContext)> destinations;

  const HomeScreen({
    super.key,
    required this.screens,
    required this.destinations,
  });

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  int _selectedIndex = 0;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: widget.screens[_selectedIndex],
      backgroundColor: ColorScheme.of(context).surface,
      bottomNavigationBar: NavigationBar(
        selectedIndex: _selectedIndex,
        onDestinationSelected: (index) {
          setState(() {
            _selectedIndex = index;
          });
        },
        destinations:
            widget.destinations
                .map((destination) => destination(context))
                .toList(),
      ),
    );
  }
}
