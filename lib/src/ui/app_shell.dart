import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../infrastructure/logging.dart';
import '../router_go.dart';
import 'panel_manager.dart';
import 'panel/animated_sidebar.dart';
import 'panel/mobile_bottom_nav.dart';

/// Main application shell with responsive layout and panel management
class AppShell extends ConsumerStatefulWidget {
  final StatefulNavigationShell navigationShell;
  
  const AppShell({super.key, required this.navigationShell});

  @override
  ConsumerState<AppShell> createState() => _AppShellState();
}

class _AppShellState extends ConsumerState<AppShell> {
  @override
  void initState() {
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    final screenWidth = MediaQuery.of(context).size.width;
    final isMobile = screenWidth <= 768;

    tempoLog.fine(
      'AppShell building with screen width: $screenWidth, isMobile: $isMobile',
    );

    // Get active navigation index from StatefulNavigationShell
    final activeIndex = widget.navigationShell.currentIndex;

    if (isMobile) {
      return _buildMobileLayout(activeIndex);
    } else {
      return _buildDesktopLayout(context, ref, activeIndex);
    }
  }

  /// Mobile layout with bottom navigation
  Widget _buildMobileLayout(int activeIndex) {
    return Scaffold(
      body: Column(
        children: [
          Expanded(child: SafeArea(bottom: false, child: widget.navigationShell)),
          MobileBottomNav(
            activeIndex: activeIndex,
            onIndexChanged: (index) {
              widget.navigationShell.goBranch(index);
            },
          ),
        ],
      ),
    );
  }

  /// Desktop layout with sidebar (simplified for traditional navigation)
  Widget _buildDesktopLayout(
    BuildContext context,
    WidgetRef ref,
    int activeIndex,
  ) {
    final shellState = ref.watch(panelManagerProvider);

    // Sidebar width constant
    const sidebarWidth = 280.0;

    return Scaffold(
      body: Stack(
        children: [
          // Main content area with conditional animated reflow
          AnimatedContainer(
            duration: const Duration(milliseconds: 300),
            curve: Curves.easeInOut,
            child: Row(
              children: [
                // Animated space for sidebar
                AnimatedContainer(
                  duration: const Duration(milliseconds: 300),
                  curve: Curves.easeInOut,
                  width: shellState.showSidebar ? sidebarWidth : 0,
                ),

                // Primary content pane (full width for traditional navigation)
                _buildMainContent(context),
              ],
            ),
          ),

          // Animated sidebar overlay
          AnimatedSidebar(
            activeIndex: activeIndex,
            onIndexChanged: (index) {
              widget.navigationShell.goBranch(index);
            },
          ),
        ],
      ),
    );
  }

  /// Main content area with consistent padding and styling
  Widget _buildMainContent(BuildContext context) {
    return Expanded(
      child: Padding(
        padding: EdgeInsets.all(
          Theme.of(context).textTheme.bodySmall?.fontSize ?? 12.0,
        ),
        child: Container(
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(
              Theme.of(context).textTheme.bodyMedium?.fontSize ?? 6.0,
            ),
          ),
          clipBehavior: Clip.hardEdge,
          child: widget.navigationShell,
        ),
      ),
    );
  }

}
