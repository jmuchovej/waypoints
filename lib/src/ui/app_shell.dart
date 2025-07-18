import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../infrastructure/logging.dart';
import '../styling/tempo_stylesheet.dart';
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
    final stylesheet = TempoStylesheet.of(ref);
    final isMobile = stylesheet.isMobile;

    tempoLog.fine(
      'AppShell building with breakpoint: ${stylesheet.breakpoint}, '
      'screen: ${stylesheet.screenWidth}x${stylesheet.screenHeight}, isMobile: $isMobile',
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
  /// Index 0: Home, Index 1: Add (handled by button), Index 2: Timers (placeholder)
  Widget _buildMobileLayout(int activeIndex) {
    return Scaffold(
      body: Column(
        children: [
          Expanded(
            child: SafeArea(bottom: false, child: widget.navigationShell),
          ),
          MobileBottomNav(
            activeIndex: activeIndex,
            onIndexChanged: (index) {
              // Handle navigation based on the new mobile structure
              if (index == 0) {
                // Go to mobile home
                widget.navigationShell.goBranch(0);
              } else if (index == 2) {
                // Timers - currently just shows snackbar
                // The actual navigation is handled in MobileBottomNav
              }
              // Add button (index 1) navigation is handled directly in _ProminentAddButton
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
            activeIndex: _adjustDesktopIndex(
              activeIndex,
            ), // Adjust for desktop navigation
            onIndexChanged: (index) {
              // Adjust index for desktop (skip mobile home branch)
              final adjustedIndex =
                  index + 1; // Skip mobile home branch (index 0)
              widget.navigationShell.goBranch(adjustedIndex);
            },
          ),
        ],
      ),
    );
  }

  /// Adjust active index for desktop sidebar (mobile home is not shown on desktop)
  int _adjustDesktopIndex(int shellIndex) {
    // Desktop sidebar shows: Tasks (0), Projects (1), Demo (2), DevTools (3)
    // Shell branches are: Home (0), Tasks (1), Projects (2), Demo (3), DevTools (4)
    // So desktop index 0 = shell index 1, desktop index 1 = shell index 2, etc.
    return shellIndex > 0 ? shellIndex - 1 : 0;
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
