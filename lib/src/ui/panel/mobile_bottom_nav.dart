import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../../flavors.dart';

/// Bottom navigation bar for mobile layout
class MobileBottomNav extends ConsumerWidget {
  const MobileBottomNav({
    super.key, 
    required this.activeIndex,
    required this.onIndexChanged,
  });

  final int activeIndex;
  final ValueChanged<int> onIndexChanged;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colorScheme = Theme.of(context).colorScheme;
    final flavor = ref.read(flavorProvider);
    final items = [
      const BottomNavigationBarItem(
        icon: Icon(Icons.format_list_bulleted_rounded),
        label: 'Tasks',
      ),
      const BottomNavigationBarItem(
        icon: Icon(Icons.folder_outlined),
        activeIcon: Icon(Icons.folder_rounded),
        label: 'Projects',
      ),
      if (flavor.isDevelopment)
        const BottomNavigationBarItem(
          icon: Icon(Icons.science_outlined),
          activeIcon: Icon(Icons.science),
          label: 'Demo',
        ),
    ];

    return BottomNavigationBar(
      currentIndex: activeIndex,
      onTap: onIndexChanged,
      items: items,
      type: BottomNavigationBarType.fixed,
      selectedItemColor: colorScheme.primary,
      unselectedItemColor: colorScheme.onSurfaceVariant,
      backgroundColor: colorScheme.surface,
    );
  }
}
