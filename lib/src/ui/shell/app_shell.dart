import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';

import '../../../flavors.dart';
import '../../infrastructure/logging.dart';
import '../../router.dart';
import '../../things/project/project_editor.dart';
import 'pane_manager.dart';

class AppShell extends StatefulWidget {
  const AppShell({super.key});

  @override
  State<AppShell> createState() => _AppShellState();
}

class _AppShellState extends State<AppShell> with TickerProviderStateMixin {
  bool _isHovering = false;
  late AnimationController _sidebarAnimationController;
  late Animation<double> _sidebarAnimation;

  @override
  Widget build(BuildContext context) {
    final router = context.router;
    final isMobile = MediaQuery.of(context).size.width <= 768;

    // Determine which tab should be active based on current route
    int activeIndex = 0;
    if (router.current.name == ProjectsRoute.name) {
      activeIndex = 1;
    } else if (F.appFlavor == Flavor.development &&
        router.current.name == DevToolsRoute.name) {
      activeIndex = 2;
    }

    tempoLog.info(
      'AppShell: Active route: ${router.current.name}, tab index: $activeIndex, mobile: $isMobile',
    );

    // On mobile, show single pane with bottom navigation
    if (isMobile) {
      return Scaffold(
        body: Column(
          children: [
            Expanded(child: SafeArea(bottom: false, child: const AutoRouter())),
            _buildBottomNavigation(context, activeIndex),
          ],
        ),
      );
    }

    // Desktop layout: multi-pane system with hover-based sidebar
    return Scaffold(
      body: Stack(
        children: [
          // Main content area
          Row(
            children: [
              // Space for sidebar when visible
              if (paneManager.showSidebar) ...[const SizedBox(width: 280)],

              // Primary content pane
              Expanded(
                child: Padding(
                  padding: EdgeInsets.all(
                    TextTheme.of(context).bodySmall?.fontSize ?? 12.0,
                  ),
                  child: Container(
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(
                        TextTheme.of(context).bodyLarge?.fontSize ?? 6.0,
                      ),
                    ),
                    clipBehavior: Clip.hardEdge,
                    child: const AutoRouter(),
                  ),
                ),
              ),

              // Detail pane (when open)
              if (paneManager.showDetailPane) ...[
                SizedBox(width: 400, child: _buildDetailPane()),
              ],
            ],
          ),

          // Hover trigger area (always present when sidebar is hidden)
          if (!paneManager.showSidebar) _buildHoverTrigger(),

          // Animated sidebar overlay
          _buildAnimatedSidebar(context, activeIndex),
        ],
      ),
    );
  }

  @override
  void dispose() {
    paneManager.removeListener(_onPaneStateChanged);
    _sidebarAnimationController.dispose();
    super.dispose();
  }

  @override
  void initState() {
    super.initState();
    // Listen to pane manager changes
    paneManager.addListener(_onPaneStateChanged);

    // Initialize animation controller for sidebar
    _sidebarAnimationController = AnimationController(
      duration: const Duration(milliseconds: 200),
      vsync: this,
    );

    _sidebarAnimation = Tween<double>(
      begin: -280.0, // Hidden position (off-screen)
      end: 0.0, // Visible position
    ).animate(
      CurvedAnimation(
        parent: _sidebarAnimationController,
        curve: Curves.easeInOut,
      ),
    );

    // Set initial animation state based on sidebar visibility
    if (paneManager.showSidebar) {
      _sidebarAnimationController.value = 1.0;
    }
  }

  Widget _buildAnimatedSidebar(BuildContext context, int activeIndex) {
    return AnimatedBuilder(
      animation: _sidebarAnimation,
      builder: (context, child) {
        return Positioned(
          left: _sidebarAnimation.value,
          top: 0,
          bottom: 0,
          width: 280,
          child: MouseRegion(
            onEnter: (_) => _onHoverEnter(),
            onExit: (_) => _onHoverExit(),
            child: DecoratedBox(
              decoration: BoxDecoration(
                // Add shadow for the floating sidebar when in hover mode
                boxShadow:
                    (!paneManager.showSidebar && _isHovering)
                        ? [
                          BoxShadow(
                            color: Colors.black.withValues(alpha: 0.1),
                            offset: const Offset(2, 0),
                            blurRadius: 8,
                          ),
                        ]
                        : null,
              ),
              child: _buildSidebarContent(context, activeIndex),
            ),
          ),
        );
      },
    );
  }

  Widget _buildBottomNavigation(BuildContext context, int activeIndex) {
    final colorScheme = Theme.of(context).colorScheme;
    final router = context.router;

    final items = [
      BottomNavigationBarItem(
        icon: Icon(Icons.format_list_bulleted_rounded),
        label: 'Tasks',
      ),
      BottomNavigationBarItem(
        icon: Icon(Icons.folder_outlined),
        activeIcon: Icon(Icons.folder_rounded),
        label: 'Projects',
      ),
      if (F.appFlavor == Flavor.development)
        BottomNavigationBarItem(
          icon: Icon(Icons.developer_mode_outlined),
          activeIcon: Icon(Icons.developer_mode),
          label: 'Dev Tools',
        ),
    ];

    return BottomNavigationBar(
      currentIndex: activeIndex,
      onTap: (index) {
        switch (index) {
          case 0:
            router.navigate(const TasksRoute());
            break;
          case 1:
            router.navigate(const ProjectsRoute());
            break;
          case 2:
            if (F.appFlavor == Flavor.development) {
              router.navigate(const DevToolsRoute());
            }
            break;
        }
      },
      items: items,
      type: BottomNavigationBarType.fixed,
      selectedItemColor: colorScheme.primary,
      unselectedItemColor: colorScheme.onSurfaceVariant,
      backgroundColor: colorScheme.surface,
    );
  }

  Widget _buildDetailPane() {
    if (paneManager.detailPaneRoute == null) {
      return const SizedBox.shrink();
    }

    // Build detail pane content based on route
    switch (paneManager.detailPaneRoute) {
      case 'project_detail':
        final uuid = paneManager.detailPaneArgs?['uuid'] as String?;
        if (uuid != null) {
          return ProjectEditor(key: ValueKey('project-editor-$uuid'));
        }
        break;
      // Add more detail pane types here
      default:
        break;
    }

    return Container(
      color: Theme.of(context).colorScheme.surface,
      child: const Center(child: Text('Detail pane')),
    );
  }

  Widget _buildHoverTrigger() {
    return Positioned(
      left: 0,
      top: 0,
      bottom: 0,
      width: 20, // Hover trigger area width
      child: MouseRegion(
        onEnter: (_) => _onHoverEnter(),
        child: Container(
          // Invisible trigger area
          color: Colors.transparent,
        ),
      ),
    );
  }

  Widget _buildNavItem(
    BuildContext context, {
    required IconData icon,
    required String label,
    required bool isSelected,
    required VoidCallback onTap,
  }) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: ListTile(
        leading: Icon(
          icon,
          color:
              isSelected
                  ? theme.colorScheme.primary
                  : theme.colorScheme.onSurfaceVariant,
        ),
        title: Text(
          label,
          style: TextStyle(
            color:
                isSelected
                    ? theme.colorScheme.primary
                    : theme.colorScheme.onSurfaceVariant,
            fontWeight: isSelected ? FontWeight.w600 : FontWeight.normal,
          ),
        ),
        selected: isSelected,
        selectedTileColor: theme.colorScheme.primaryContainer.withValues(
          alpha: 0.3,
        ),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        onTap: onTap,
      ),
    );
  }

  Widget _buildSidebarContent(BuildContext context, int activeIndex) {
    final theme = Theme.of(context);
    final router = context.router;

    return Container(
      width: 280,
      decoration: BoxDecoration(color: theme.colorScheme.surfaceContainer),
      child: Column(
        children: [
          // App header
          Container(
            height: 80,
            padding: const EdgeInsets.all(24),
            child: Row(
              children: [
                Icon(
                  Icons.music_note_rounded,
                  size: 32,
                  color: theme.colorScheme.primary,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    'Tempo',
                    style: theme.textTheme.headlineSmall?.copyWith(
                      fontWeight: FontWeight.bold,
                      color: theme.colorScheme.primary,
                    ),
                  ),
                ),
                // Add a pin/unpin button for permanent sidebar toggle
                IconButton(
                  icon: Icon(
                    paneManager.showSidebar
                        ? Icons.push_pin
                        : Icons.push_pin_outlined,
                  ),
                  onPressed: () => paneManager.toggleSidebar(),
                  tooltip:
                      paneManager.showSidebar
                          ? 'Auto-hide sidebar'
                          : 'Pin sidebar',
                ),
              ],
            ),
          ),

          // Navigation items
          Expanded(
            child: ListView(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              children: [
                _buildNavItem(
                  context,
                  icon: Icons.format_list_bulleted_rounded,
                  label: 'Tasks',
                  isSelected: activeIndex == 0,
                  onTap: () => router.navigate(const TasksRoute()),
                ),
                _buildNavItem(
                  context,
                  icon: Icons.folder_rounded,
                  label: 'Projects',
                  isSelected: activeIndex == 1,
                  onTap: () => router.navigate(const ProjectsRoute()),
                ),
                if (F.appFlavor == Flavor.development) ...[
                  const SizedBox(height: 24),
                  Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 8,
                    ),
                    child: Text(
                      'Development',
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  _buildNavItem(
                    context,
                    icon: Icons.developer_mode,
                    label: 'Dev Tools',
                    isSelected: activeIndex == 2,
                    onTap: () => router.navigate(const DevToolsRoute()),
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }

  void _onHoverEnter() {
    if (!paneManager.showSidebar) {
      setState(() {
        _isHovering = true;
      });
      _sidebarAnimationController.forward();
    }
  }

  void _onHoverExit() {
    if (!paneManager.showSidebar) {
      setState(() {
        _isHovering = false;
      });
      // Delay hiding to allow moving between trigger and sidebar
      Future.delayed(const Duration(milliseconds: 100), () {
        if (!_isHovering && !paneManager.showSidebar) {
          _sidebarAnimationController.reverse();
        }
      });
    }
  }

  void _onPaneStateChanged() {
    // Update animation based on sidebar visibility
    if (paneManager.showSidebar) {
      _sidebarAnimationController.forward();
    } else {
      _sidebarAnimationController.reverse();
    }
    setState(() {});
  }
}
