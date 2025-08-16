import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../infrastructure/logging.dart';
import '../infrastructure/sidebar_persistence.dart';
import 'panel_request.dart';
import 'panel_content_registry.dart';

/// State for a single panel
class PanelState {
  const PanelState({
    required this.id,
    required this.entityUUID,
    required this.entityType,
    required this.openedAt,
    this.metadata,
  });

  final String id;
  final String entityUUID;
  final String entityType;
  final DateTime openedAt;
  final Map<String, dynamic>? metadata;
}

/// Overall panel system state including sidebar and detail pane
class PanelSystemState {
  const PanelSystemState({
    this.openPanels = const [],
    this.activePanelID,
    this.isVisible = false,
    this.showSidebar = true,
    this.splitRatio = 0.65, // Default 65/35 split for main/panel
    this.panelWidth = 400.0, // Default panel width (used as minimum)
    this.isResizing = false, // Whether panel is currently being resized
  });

  final List<PanelState> openPanels;
  final String? activePanelID;
  final bool isVisible;
  final bool showSidebar;
  final double splitRatio; // Ratio of main content (0.0-1.0)
  final double panelWidth; // Current panel width
  final bool isResizing; // Whether panel is currently being resized

  PanelSystemState copyWith({
    List<PanelState>? openPanels,
    String? activePanelID,
    bool? isVisible,
    bool? showSidebar,
    double? splitRatio,
    double? panelWidth,
    bool? isResizing,
  }) {
    return PanelSystemState(
      openPanels: openPanels ?? this.openPanels,
      activePanelID: activePanelID ?? this.activePanelID,
      isVisible: isVisible ?? this.isVisible,
      showSidebar: showSidebar ?? this.showSidebar,
      splitRatio: splitRatio ?? this.splitRatio,
      panelWidth: panelWidth ?? this.panelWidth,
      isResizing: isResizing ?? this.isResizing,
    );
  }
}

/// Unified panel manager that handles all shell state (panels, sidebar, detail pane)
class PanelManager extends StateNotifier<PanelSystemState> {
  PanelManager(this._ref) : super(const PanelSystemState()) {
    _requestSubscription = _requestController.stream.listen(_handleRequest);
    _loadPersistedSidebarState();
  }
  final Ref _ref;
  final _requestController = StreamController<PanelRequest>.broadcast();
  late final StreamSubscription<PanelRequest> _requestSubscription;

  /// Emit a panel request
  void emit(PanelRequest request) {
    waypointsLog.fine('PanelManager: Emitting request: $request');
    _requestController.add(request);
  }

  /// Handle different request types
  void _handleRequest(PanelRequest request) {
    waypointsLog.finest('Received: $request');
    switch (request) {
      case OpenThingRequest():
        _handleOpenThing(request);
      case ClosePanelRequest():
        _handleClosePanel(request);
      case SwitchPanelRequest():
        _handleSwitchPanel(request);
      case ToggleSidebarRequest():
        _handleToggleSidebar(request);
    }
  }

  void _handleOpenThing(OpenThingRequest request, [BuildContext? context]) {
    final registry = _ref.read(panelContentRegistryProvider);

    // Check if we have a builder for this entity type
    if (!registry.hasBuilder(request.type)) {
      waypointsLog.warning(
        'No panel builder registered for entity type: ${request.type}',
      );
      return;
    }

    // Get panel configuration
    final builder = registry.getBuilder(request.type)!;
    final config = builder.getDefaultConfig();

    // Check screen size to determine strategy
    // Context should be provided by the calling extension method
    BuildContext? detectionContext = context;

    if (detectionContext != null) {
      final screenWidth = MediaQuery.of(detectionContext).size.width;
      final isMobile = screenWidth < 768;
      waypointsLog.finest('📱 Screen width: ${screenWidth}px, isMobile: $isMobile');

      if (isMobile) {
        waypointsLog.info('📱 Mobile detected - routing to full screen navigation');
        // On mobile, navigate to full screen instead
        _navigateToFullScreen(request, detectionContext);
        return;
      } else {
        waypointsLog.finest('🖥️ Desktop detected - using panel system');
      }
    } else {
      waypointsLog.warning(
        '❌ No context available for mobile detection - defaulting to panel system',
      );
    }

    // Desktop: open in panel
    final panelID = 'panel-${request.uuid}';
    final newPanel = PanelState(
      id: panelID,
      entityUUID: request.uuid,
      entityType: request.type,
      openedAt: DateTime.now(),
      metadata: request.metadata,
    );

    if (config.replaceExisting) {
      // Replace all existing panels
      state = state.copyWith(
        openPanels: [newPanel],
        activePanelID: panelID,
        isVisible: true,
      );
    } else if (config.allowMultiple) {
      // Add to existing panels
      state = state.copyWith(
        openPanels: [...state.openPanels, newPanel],
        activePanelID: panelID,
        isVisible: true,
      );
    } else {
      // Single panel mode - replace if exists
      final existingIndex = state.openPanels.indexWhere(
        (p) => p.entityType == request.type,
      );

      if (existingIndex >= 0) {
        final updatedPanels = [...state.openPanels];
        updatedPanels[existingIndex] = newPanel;
        state = state.copyWith(
          openPanels: updatedPanels,
          activePanelID: panelID,
          isVisible: true,
        );
      } else {
        state = state.copyWith(
          openPanels: [...state.openPanels, newPanel],
          activePanelID: panelID,
          isVisible: true,
        );
      }
    }

    waypointsLog.fine('Opened panel for ${request.type}: ${request.uuid}');
  }

  void _handleClosePanel(ClosePanelRequest request) {
    if (request.panelID != null) {
      // Close specific panel
      state = state.copyWith(
        openPanels:
            state.openPanels.where((p) => p.id != request.panelID).toList(),
      );
    } else {
      // Close all panels
      state = const PanelSystemState();
    }
  }

  void _handleSwitchPanel(SwitchPanelRequest request) {
    if (state.openPanels.any((p) => p.id == request.panelID)) {
      state = state.copyWith(activePanelID: request.panelID);
    }
  }

  void _navigateToFullScreen(OpenThingRequest request, BuildContext context) {
    waypointsLog.info(
      '🚀 PanelManager routing to full-screen for ${request.type}: ${request.uuid}',
    );

    try {
      // Use AutoRoute to navigate to full-screen entity view
      context.go('/thing/${request.uuid}');
      waypointsLog.info('✅ Full-screen navigation completed successfully');
    } catch (e, stack) {
      waypointsLog.severe('❌ Full-screen navigation failed: $e', e, stack);
    }
  }

  /// Handle sidebar toggle requests
  void _handleToggleSidebar(ToggleSidebarRequest request) {
    final newState = !state.showSidebar;
    state = state.copyWith(showSidebar: newState);
    waypointsLog.fine('Sidebar toggled: $newState');
    _saveSidebarState(newState);
  }

  /// Update the split ratio when panel is resized
  void updateSplitRatio(double ratio) {
    state = state.copyWith(splitRatio: ratio.clamp(0.3, 0.8));
  }

  /// Update panel width (for resize tracking)
  void updatePanelWidth(double width) {
    state = state.copyWith(panelWidth: width);
  }

  /// Set resize state (to disable animations during resize)
  void setResizing(bool isResizing) {
    state = state.copyWith(isResizing: isResizing);
  }

  /// Batch update both panel dimensions for better performance
  void updatePanelDimensions(double splitRatio, double panelWidth) {
    state = state.copyWith(
      splitRatio: splitRatio.clamp(0.3, 0.8),
      panelWidth: panelWidth,
    );
  }

  /// Direct methods for convenience (non-request based)
  void toggleSidebar() {
    emit(ToggleSidebarRequest());
  }

  void setSidebarVisible(bool visible) {
    if (state.showSidebar != visible) {
      state = state.copyWith(showSidebar: visible);
      waypointsLog.fine('Sidebar visibility set to: $visible');
      _saveSidebarState(visible);
    }
  }

  void openDetailPane(String route, {Map<String, dynamic>? args}) {
    emit(OpenDetailPaneRequest(route: route, args: args));
  }

  void closeDetailPane() {
    emit(CloseDetailPaneRequest());
  }

  /// Load persisted sidebar state on initialization
  Future<void> _loadPersistedSidebarState() async {
    try {
      final persistedState = await SidebarPersistence.loadSidebarState();
      state = state.copyWith(showSidebar: persistedState);
      waypointsLog.fine('Loaded persisted sidebar state: $persistedState');
    } catch (e, stack) {
      waypointsLog.warning('Failed to load persisted sidebar state', e, stack);
    }
  }

  /// Save sidebar state to persistence
  void _saveSidebarState(bool showSidebar) {
    // Save asynchronously without blocking UI
    SidebarPersistence.saveSidebarState(showSidebar).catchError((e, stack) {
      waypointsLog.warning('Failed to save sidebar state', e, stack);
    });
  }

  @override
  void dispose() {
    _requestSubscription.cancel();
    _requestController.close();
    super.dispose();
  }
}

/// Provider for the panel manager
final panelManagerProvider =
    StateNotifierProvider<PanelManager, PanelSystemState>((ref) {
      return PanelManager(ref);
    });

// Note: navigatorKeyProvider removed - navigation now handled directly through context

/// Extension for easy panel and shell requests
extension PanelRequests on WidgetRef {
  void openThingPanel(String uuid, String entityType) {
    read(
      panelManagerProvider.notifier,
    ).emit(OpenThingRequest(uuid: uuid, type: entityType));
  }

  void closePanel([String? panelId]) {
    read(
      panelManagerProvider.notifier,
    ).emit(ClosePanelRequest(panelID: panelId));
  }

  void toggleShellSidebar() {
    read(panelManagerProvider.notifier).toggleSidebar();
  }

  void openShellDetailPane(String route, {Map<String, dynamic>? args}) {
    read(panelManagerProvider.notifier).openDetailPane(route, args: args);
  }

  void closeShellDetailPane() {
    read(panelManagerProvider.notifier).closeDetailPane();
  }

  void updatePanelSplitRatio(double ratio) {
    read(panelManagerProvider.notifier).updateSplitRatio(ratio);
  }
}
