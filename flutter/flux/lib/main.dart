import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flux/utils/logging.dart';
import 'package:flux/objectbox.dart';
import 'package:flux/ui/screens/tasks.dart';
import 'package:flux/ui/screens/projects.dart';

import 'package:flux/dev/stub.dart'
    if (dart.library.io) 'package:flux/dev/tools.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await ObjectBox.create();

  if (kDebugMode) {
    await initDevEnvironment();
  }

  runApp(TempoFlux());
}

class TempoFlux extends StatelessWidget {
  const TempoFlux({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Flux',
      theme: ThemeData(
        primarySwatch: Colors.blue,
        visualDensity: VisualDensity.adaptivePlatformDensity,
        useMaterial3: true,
      ),
      home: MainScreen(),
    );
  }
}

class MainScreen extends StatefulWidget {
  const MainScreen({super.key});

  @override
  _MainScreenState createState() => _MainScreenState();
}

class _MainScreenState extends State<MainScreen> {
  int _selectedIndex = 0;

  @override
  Widget build(BuildContext context) {
    final List<Widget> screens = [TasksScreen(), ProjectsScreen()];

    if (kDebugMode) {
      addDevTools(screens);
    }

    return Scaffold(
      body: screens[_selectedIndex],
      bottomNavigationBar: NavigationBar(
        selectedIndex: _selectedIndex,
        onDestinationSelected: (index) {
          setState(() {
            _selectedIndex = index;
          });
        },
        destinations: [
          NavigationDestination(
            icon: Icon(Icons.home_outlined, color: Colors.grey.shade400),
            selectedIcon: Icon(
              Icons.home_rounded,
              color: Theme.of(context).primaryColor,
            ),
            label: 'Tasks',
          ),
          NavigationDestination(
            icon: Icon(Icons.folder_outlined),
            selectedIcon: Icon(Icons.folder),
            label: 'Projects',
          ),
          getDevToolsDestination(context),
        ],
      ),
    );
  }
}
