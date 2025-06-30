import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../flavors.dart';
import '../panel_manager.dart';

/// Animated sidebar with hover trigger and pin functionality
class AnimatedSidebar extends ConsumerStatefulWidget {
  const AnimatedSidebar({
    super.key,
    required this.activeIndex,
    required this.onIndexChanged,
  });

  final int activeIndex;
  final ValueChanged<int> onIndexChanged;

  @override
  ConsumerState<AnimatedSidebar> createState() => _AnimatedSidebarState();
}

class _AnimatedSidebarState extends ConsumerState<AnimatedSidebar>
    with SingleTickerProviderStateMixin {
  bool _isHovering = false;
  late AnimationController _animationController;
  late Animation<double> _slideAnimation;

  @override
  void initState() {
    super.initState();

    _animationController = AnimationController(
      duration: const Duration(milliseconds: 200),
      vsync: this,
    );

    _slideAnimation = Tween<double>(begin: -280.0, end: 0.0).animate(
      CurvedAnimation(parent: _animationController, curve: Curves.easeInOut),
    );

    // Set initial state based on panel manager
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final shellState = ref.read(panelManagerProvider);
      if (shellState.showSidebar) {
        _animationController.value = 1.0;
      }
    });
  }

  @override
  void dispose() {
    _animationController.dispose();
    super.dispose();
  }

  void _onHoverEnter() {
    final shellState = ref.read(panelManagerProvider);
    if (!shellState.showSidebar) {
      setState(() => _isHovering = true);
      _animationController.forward();
    }
  }

  void _onHoverExit() {
    final shellState = ref.read(panelManagerProvider);
    if (!shellState.showSidebar) {
      setState(() => _isHovering = false);
      Future.delayed(const Duration(milliseconds: 100), () {
        if (mounted && !_isHovering && !ref.read(panelManagerProvider).showSidebar) {
          _animationController.reverse();
        }
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final shellState = ref.watch(panelManagerProvider);

    // React to sidebar state changes
    ref.listen(panelManagerProvider, (previous, current) {
      if (previous?.showSidebar != current.showSidebar) {
        if (current.showSidebar) {
          _animationController.forward();
        } else {
          _animationController.reverse();
        }
      }
    });

    return Stack(
      children: [
        // Hover trigger area
        if (!shellState.showSidebar)
          Positioned(
            left: 0,
            top: 0,
            bottom: 0,
            width: 20,
            child: MouseRegion(
              onEnter: (_) => _onHoverEnter(),
              child: IgnorePointer(child: Container(color: Colors.transparent)),
            ),
          ),

        // Animated sidebar
        AnimatedBuilder(
          animation: _slideAnimation,
          builder: (context, child) {
            return Positioned(
              left: _slideAnimation.value,
              top: 0,
              bottom: 0,
              width: 280,
              child: MouseRegion(
                onEnter: (_) => _onHoverEnter(),
                onExit: (_) => _onHoverExit(),
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    boxShadow:
                        (!shellState.showSidebar && _isHovering)
                            ? [
                              BoxShadow(
                                color: Colors.black.withOpacity(0.1),
                                offset: const Offset(2, 0),
                                blurRadius: 8,
                              ),
                            ]
                            : null,
                  ),
                  child: _SidebarContent(
                    activeIndex: widget.activeIndex,
                    onIndexChanged: widget.onIndexChanged,
                    shellState: shellState,
                    onToggleSidebar:
                        () =>
                            ref
                                .read(panelManagerProvider.notifier)
                                .toggleSidebar(),
                  ),
                ),
              ),
            );
          },
        ),
      ],
    );
  }
}

/// Sidebar content with navigation items
class _SidebarContent extends ConsumerWidget {
  const _SidebarContent({
    required this.activeIndex,
    required this.onIndexChanged,
    required this.shellState,
    required this.onToggleSidebar,
  });

  final int activeIndex;
  final ValueChanged<int> onIndexChanged;
  final PanelSystemState shellState;
  final VoidCallback onToggleSidebar;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final flavor = ref.watch(flavorProvider);

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
                IconButton(
                  icon: Icon(
                    shellState.showSidebar
                        ? Icons.push_pin
                        : Icons.push_pin_outlined,
                  ),
                  onPressed: onToggleSidebar,
                  tooltip:
                      shellState.showSidebar
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
                _NavItem(
                  icon: Icons.format_list_bulleted_rounded,
                  label: 'Tasks',
                  isSelected: activeIndex == 0,
                  onTap: () {
                    onIndexChanged(0);
                  },
                ),
                _NavItem(
                  icon: Icons.folder_rounded,
                  label: 'Projects',
                  isSelected: activeIndex == 1,
                  onTap: () {
                    onIndexChanged(1);
                  },
                ),
                if (flavor.isDevelopment) ...[
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
                  _NavItem(
                    icon: Icons.science,
                    label: 'Demo',
                    isSelected: activeIndex == 2,
                    onTap: () {
                      onIndexChanged(2);
                    },
                  ),
                  _NavItem(
                    icon: Icons.developer_board,
                    label: 'DevTools',
                    isSelected: activeIndex == 3,
                    onTap: () {
                      onIndexChanged(3);
                    },
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// Individual navigation item
class _NavItem extends StatelessWidget {
  const _NavItem({
    required this.icon,
    required this.label,
    required this.isSelected,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final bool isSelected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
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
        selectedTileColor: theme.colorScheme.primaryContainer.withOpacity(0.3),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        onTap: onTap,
      ),
    );
  }
}
