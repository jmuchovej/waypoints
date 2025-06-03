import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:tempo/flavors.dart';

class HomeShell extends StatelessWidget {
  final Widget child;

  const HomeShell({super.key, required this.child});

  int _calculateSelectedIndex(BuildContext context) {
    final String location = GoRouterState.of(context).uri.toString();
    if (location.startsWith('/tasks')) {
      return 0;
    }
    if (location.startsWith('/projects')) {
      return 1;
    }
    if (location.startsWith('/devtools')) {
      return 2;
    }
    return 0;
  }

  void _onDestinationSelected(BuildContext context, int index) {
    switch (index) {
      case 0:
        context.go('/tasks');
        break;
      case 1:
        context.go('/projects');
        break;
      case 2:
        if (F.appFlavor == Flavor.development) {
          context.go('/devtools');
        }
        break;
    }
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final selectedIndex = _calculateSelectedIndex(context);

    return Scaffold(
      body: child,
      backgroundColor: colorScheme.surface,
      bottomNavigationBar: NavigationBar(
        selectedIndex: selectedIndex,
        onDestinationSelected:
            (index) => _onDestinationSelected(context, index),
        destinations: [
          NavigationDestination(
            icon: Icon(
              Icons.format_list_bulleted_rounded,
              color: colorScheme.primaryContainer,
            ),
            selectedIcon: Icon(
              Icons.format_list_bulleted_rounded,
              color: colorScheme.primary,
            ),
            label: 'Tasks',
          ),
          NavigationDestination(
            icon: Icon(
              Icons.folder_outlined,
              color: colorScheme.primaryContainer,
            ),
            selectedIcon: Icon(
              Icons.folder_rounded,
              color: colorScheme.primary,
            ),
            label: 'Projects',
          ),
          if (F.appFlavor == Flavor.development) ...[
            NavigationDestination(
              icon: Icon(
                Icons.developer_mode_outlined,
                color: colorScheme.primaryContainer,
              ),
              selectedIcon: Icon(
                Icons.developer_mode,
                color: colorScheme.primary,
              ),
              label: 'Dev Tools',
            ),
          ],
        ],
      ),
    );
  }
}
