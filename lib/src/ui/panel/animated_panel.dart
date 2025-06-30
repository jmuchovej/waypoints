import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../panel_content_registry.dart';
import '../panel_manager.dart';
import '../panel_request.dart';
import 'resize_controller.dart';

/// Animated panel with slide-in/out animation and resize support
class AnimatedPanel extends ConsumerStatefulWidget {
  const AnimatedPanel({super.key, required this.width});

  final double width;

  @override
  ConsumerState<AnimatedPanel> createState() => _AnimatedPanelState();
}

class _AnimatedPanelState extends ConsumerState<AnimatedPanel>
    with SingleTickerProviderStateMixin {
  late AnimationController _animationController;
  late Animation<double> _slideAnimation;
  ResizeController? _resizeController;

  bool _isResizing = false;

  @override
  void initState() {
    super.initState();

    _animationController = AnimationController(
      duration: const Duration(milliseconds: 300),
      vsync: this,
    );

    _slideAnimation = Tween<double>(begin: 1.0, end: 0.0).animate(
      CurvedAnimation(parent: _animationController, curve: Curves.easeInOut),
    );

    // ResizeController will be initialized in build method
  }

  @override
  void dispose() {
    _animationController.dispose();
    _resizeController?.dispose();
    super.dispose();
  }

  void _updateAnimation(PanelSystemState panelState) {
    final shouldShow = panelState.isVisible && panelState.openPanels.isNotEmpty;

    if (shouldShow && !_animationController.isCompleted) {
      _animationController.forward();
    } else if (!shouldShow && !_animationController.isDismissed) {
      _animationController.reverse();
    }
  }

  @override
  Widget build(BuildContext context) {
    final panelState = ref.watch(panelManagerProvider);
    _updateAnimation(panelState);

    // Initialize ResizeController if not already done
    _resizeController ??= ref.read(resizeControllerProvider);

    // Use width from props directly for better performance
    return AnimatedBuilder(
      animation: _slideAnimation,
      builder: (context, child) {
        final allocatedWidth = widget.width * (1.0 - _slideAnimation.value);

        if (allocatedWidth <= 0.0) {
          return const SizedBox.shrink();
        }

        return SizedBox(
          width: allocatedWidth,
          child: ClipRect(
            child: Transform.translate(
              offset: Offset(widget.width - allocatedWidth, 0),
              child: SizedBox(
                width: widget.width,
                child: _PanelContent(
                  panelState: panelState,
                  panelWidth: widget.width,
                  isResizing: _isResizing,
                  onResizeStart: () {
                    setState(() => _isResizing = true);
                    _resizeController?.startResize();
                  },
                  onResizeUpdate: (DragUpdateDetails details) {
                    final screenWidth = MediaQuery.of(context).size.width;
                    const sidebarWidth = 280.0;
                    final availableWidth =
                        screenWidth -
                        (panelState.showSidebar ? sidebarWidth : 0);

                    _resizeController?.updateResize(
                      screenWidth: screenWidth,
                      availableWidth: availableWidth,
                      currentPanelWidth: widget.width,
                      dragDelta: details.delta.dx,
                      showSidebar: panelState.showSidebar,
                    );
                  },
                  onResizeEnd: () {
                    setState(() => _isResizing = false);
                    _resizeController?.endResize();
                  },
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

/// Panel content with header and resize handle
class _PanelContent extends ConsumerWidget {
  const _PanelContent({
    required this.panelState,
    required this.panelWidth,
    required this.isResizing,
    required this.onResizeStart,
    required this.onResizeUpdate,
    required this.onResizeEnd,
  });

  final PanelSystemState panelState;
  final double panelWidth;
  final bool isResizing;
  final VoidCallback onResizeStart;
  final ValueChanged<DragUpdateDetails> onResizeUpdate;
  final VoidCallback onResizeEnd;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    if (panelState.openPanels.isEmpty) {
      return const SizedBox.shrink();
    }

    final activePanel = panelState.openPanels.firstWhere(
      (p) => p.id == panelState.activePanelID,
      orElse: () => panelState.openPanels.last,
    );

    return Stack(
      children: [
        // Panel content
        DecoratedBox(
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.surface,
            boxShadow: [
              BoxShadow(
                color: Colors.black.withOpacity(0.05),
                blurRadius: 4,
                offset: const Offset(-2, 0),
              ),
            ],
          ),
          child: Column(
            children: [
              // Panel content with performance optimization during resize
              Expanded(
                child: RepaintBoundary(
                  child: IgnorePointer(
                    ignoring: isResizing, // Prevent interactions during resize
                    child: AnimatedSwitcher(
                      duration: isResizing 
                        ? Duration.zero 
                        : const Duration(milliseconds: 200),
                      child: _CachedPanelContent(
                        key: ValueKey('${activePanel.entityType}-${activePanel.entityUUID}'),
                        entityType: activePanel.entityType,
                        entityUUID: activePanel.entityUUID,
                        isResizing: isResizing,
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
        // Resize handle
        Positioned(
          left: 0,
          top: 0,
          bottom: 0,
          child: _ResizeHandle(
            isResizing: isResizing,
            onResizeStart: onResizeStart,
            onResizeUpdate: onResizeUpdate,
            onResizeEnd: onResizeEnd,
          ),
        ),
      ],
    );
  }
}

/// Panel header with title and close button
class _PanelHeader extends ConsumerWidget {
  const _PanelHeader({required this.panel});

  final PanelState panel;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Container(
      height: 48,
      padding: const EdgeInsets.symmetric(horizontal: 16),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceVariant,
        border: Border(
          bottom: BorderSide(color: Theme.of(context).dividerColor),
        ),
      ),
      child: Row(
        children: [
          Icon(_getIconForType(panel.entityType), size: 20),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              _formatType(panel.entityType),
              style: Theme.of(context).textTheme.titleSmall,
            ),
          ),
          IconButton(
            icon: const Icon(Icons.close, size: 20),
            onPressed:
                () => ref
                    .read(panelManagerProvider.notifier)
                    .emit(ClosePanelRequest(panelID: panel.id)),
            tooltip: 'Close panel',
          ),
        ],
      ),
    );
  }

  IconData _getIconForType(String type) {
    switch (type) {
      case 'task':
        return Icons.task_alt;
      case 'project':
        return Icons.folder;
      default:
        return Icons.article;
    }
  }

  String _formatType(String type) {
    return type[0].toUpperCase() + type.substring(1);
  }
}

/// Cached panel content that avoids rebuilds during resize operations
class _CachedPanelContent extends ConsumerStatefulWidget {
  const _CachedPanelContent({
    super.key,
    required this.entityType,
    required this.entityUUID,
    required this.isResizing,
  });

  final String entityType;
  final String entityUUID;
  final bool isResizing;

  @override
  ConsumerState<_CachedPanelContent> createState() => _CachedPanelContentState();
}

class _CachedPanelContentState extends ConsumerState<_CachedPanelContent> {
  Widget? _cachedContent;
  String? _lastEntityKey;

  @override
  Widget build(BuildContext context) {
    final entityKey = '${widget.entityType}-${widget.entityUUID}';
    
    // Only rebuild content if entity changed or not currently resizing
    if (!widget.isResizing && (_cachedContent == null || _lastEntityKey != entityKey)) {
      _cachedContent = ref
          .read(panelContentRegistryProvider)
          .buildContent(
            context,
            widget.entityType,
            widget.entityUUID,
            ref,
          );
      _lastEntityKey = entityKey;
    }
    
    // During resize, return cached content to avoid expensive rebuilds
    return _cachedContent ?? const Center(child: CircularProgressIndicator());
  }
}

/// Resize handle for panel width adjustment
class _ResizeHandle extends StatelessWidget {
  const _ResizeHandle({
    required this.isResizing,
    required this.onResizeStart,
    required this.onResizeUpdate,
    required this.onResizeEnd,
  });

  final bool isResizing;
  final VoidCallback onResizeStart;
  final ValueChanged<DragUpdateDetails> onResizeUpdate;
  final VoidCallback onResizeEnd;

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      cursor: SystemMouseCursors.resizeLeftRight,
      child: GestureDetector(
        onPanStart: (_) => onResizeStart(),
        onPanUpdate: onResizeUpdate,
        onPanEnd: (_) => onResizeEnd(),
        child: Container(
          width: 8,
          decoration: BoxDecoration(
            color:
                isResizing
                    ? Theme.of(context).colorScheme.primary.withOpacity(0.3)
                    : Colors.transparent,
            border: Border(
              right: BorderSide(
                color: Theme.of(context).dividerColor,
                width: 1,
              ),
            ),
          ),
          child: Center(
            child: Container(
              width: 2,
              height: 24,
              decoration: BoxDecoration(
                color: Theme.of(context).colorScheme.outline.withOpacity(0.5),
                borderRadius: BorderRadius.circular(1),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
