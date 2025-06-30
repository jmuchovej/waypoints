import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../routes/thing/entity_page.dart';
import '../../infrastructure/logging.dart';
import '../panel_manager.dart';

/// Universal page wrapper that displays any base page with animated panels
class UniversalPanelPage extends ConsumerStatefulWidget {
  const UniversalPanelPage({
    super.key,
    required this.basePage,
    required this.panels,
  });

  final Widget basePage;
  final List<String> panels;

  @override
  ConsumerState<UniversalPanelPage> createState() => _UniversalPanelPageState();
}

class _UniversalPanelPageState extends ConsumerState<UniversalPanelPage> with TickerProviderStateMixin {
  final ScrollController _scrollController = ScrollController();
  AnimationController? _slideAnimationController;
  Animation<double>? _slideAnimation;
  int _animatingPanelIndex = -1;
  double _baseContentWidth = 0.0;
  double _panelWidth = 0.0;

  @override
  void initState() {
    super.initState();

    _slideAnimationController = AnimationController(
      duration: const Duration(milliseconds: 300), // Match scroll animation speed
      vsync: this,
    );

    _slideAnimation = Tween<double>(begin: 1.0, end: 0.0).animate(
      CurvedAnimation(parent: _slideAnimationController!, curve: Curves.easeOut),
    );

    // Check if we should animate the initial panel
    // Only animate if this is the first panel being added to an empty state
    if (widget.panels.length == 1) {
      _animatingPanelIndex = 0;
      _slideAnimationController!.forward();
    }
  }

  @override
  void didUpdateWidget(UniversalPanelPage oldWidget) {
    super.didUpdateWidget(oldWidget);

    // Handle panel changes from navigation
    if (widget.panels.length > oldWidget.panels.length) {
      // New panel added - animate it sliding in
      _animatingPanelIndex = widget.panels.length - 1;
      _slideAnimationController!.reset();
      _slideAnimationController!.forward();

      tempoLog.info('UniversalPanelPage: Animating slide-in for panel at index $_animatingPanelIndex');

      // Scroll to show new panel after animation
      Future.delayed(const Duration(milliseconds: 350), () {
        if (mounted && _scrollController.hasClients) {
          _scrollController.animateTo(
            _scrollController.position.maxScrollExtent,
            duration: const Duration(milliseconds: 300),
            curve: Curves.easeOut,
          );
        }
      });
    } else if (widget.panels.length == oldWidget.panels.length) {
      // Same number of panels - check if content changed
      bool contentChanged = false;
      for (int i = 0; i < widget.panels.length; i++) {
        if (i >= oldWidget.panels.length || widget.panels[i] != oldWidget.panels[i]) {
          contentChanged = true;
          break;
        }
      }
      
      if (contentChanged) {
        tempoLog.info('UniversalPanelPage: Panel content changed, refreshing without animation');
        // Just reset animation state - no need for setState since panels prop changed
        _animatingPanelIndex = -1;
      } else {
        _animatingPanelIndex = -1;
      }
    } else {
      // Panels removed - no animation needed
      _animatingPanelIndex = -1;
    }
  }

  @override
  void dispose() {
    _slideAnimationController?.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  void _calculateWidths(double screenWidth, bool showSidebar) {
    const sidebarWidth = 280.0; // From app_shell.dart
    const padding = 24.0 * 2; // Left and right padding
    final sidebarSpace = showSidebar ? sidebarWidth : 0.0;
    final availableWidth = screenWidth - sidebarSpace - padding;

    _baseContentWidth = availableWidth * 0.6; // 60% for base content
    _panelWidth = availableWidth * 0.4;       // 40% for each panel
  }

  @override
  Widget build(BuildContext context) {
    // If no panels, just show the base page
    if (widget.panels.isEmpty) {
      return widget.basePage;
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        // Get sidebar state for width calculations
        final shellState = ref.watch(panelManagerProvider);
        final showSidebar = shellState.showSidebar;
        
        // Calculate widths based on available space minus sidebar
        _calculateWidths(MediaQuery.of(context).size.width, showSidebar);

        return SingleChildScrollView(
          controller: _scrollController,
          scrollDirection: Axis.horizontal,
          reverse: true,
          physics: const ClampingScrollPhysics(),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.start,
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.max,
            children: [
              // Base content - always visible
              Container(
                width: _baseContentWidth,
                decoration: BoxDecoration(
                  border: Border(
                    right: BorderSide(color: Colors.grey.shade300),
                  ),
                ),
                child: widget.basePage,
              ),
              // Gap between base content and first panel
              const SizedBox(width: 12),
              // Panel list with gaps between panels
              ...widget.panels.asMap().entries.expand((entry) {
                final index = entry.key;
                final uuid = entry.value;
                return [
                  _buildPanelItem(uuid, index),
                  // Gap after each panel (except the last one)
                  if (index < widget.panels.length - 1) const SizedBox(width: 12),
                ];
              }),
            ],
          ),
        );
      },
    );
  }

  Widget _buildPanelItem(String uuid, int index) {
    final panelContent = Container(
      width: _panelWidth,
      decoration: BoxDecoration(
        color: Colors.white,
        border: Border(
          left: BorderSide(color: Colors.grey.shade300, width: 2),
        ),
      ),
      child: EntityPage(
        uuid: uuid,
        panelInfo: PanelInfo.panel(index + 1, uuid), // Panel 1, 2, 3, etc.
      ),
    );

    // Apply slide-in animation only to the newest panel
    if (index == _animatingPanelIndex && _slideAnimation != null) {
      return AnimatedBuilder(
        animation: _slideAnimation!,
        builder: (context, child) {
          final slideValue = _slideAnimation!.value;
          final allocatedWidth = _panelWidth * (1.0 - slideValue);

          if (allocatedWidth <= 0.0) {
            return const SizedBox.shrink();
          }

          return SizedBox(
            width: allocatedWidth,
            child: ClipRect(
              child: Transform.translate(
                offset: Offset(_panelWidth - allocatedWidth, 0),
                child: panelContent,
              ),
            ),
          );
        },
      );
    }

    // Static panels (no animation)
    return panelContent;
  }
}

