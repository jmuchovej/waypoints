import 'dart:async';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../infrastructure/logging.dart';
import '../panel_manager.dart';

/// Controller for managing panel resize operations with direct pixel-based updates
/// Eliminates percentage calculations during drag for smooth resize tracking
class ResizeController {
  ResizeController({
    required this.ref,
    this.debounceMs = 16, // ~60fps for smooth updates (was over-optimized)
    this.persistDelayMs = 50, // Reduced delay since calculations are simpler
    this.minUpdateDelta = 1.0, // Reduced threshold for more responsive tracking
  });

  final Ref ref;
  final int debounceMs;
  final int persistDelayMs;
  final double minUpdateDelta;

  Timer? _updateTimer;
  Timer? _persistTimer;
  double? _pendingPanelWidth;
  double? _lastUpdatedWidth; // Track last width to avoid redundant updates
  bool _isResizing = false;

  // Performance monitoring
  DateTime? _resizeStartTime;
  int _updateCount = 0;
  int _persistCount = 0;

  /// Current panel manager
  PanelManager get panelManager => ref.read(panelManagerProvider.notifier);

  /// Start resize operation
  void startResize() {
    _isResizing = true;
    _resizeStartTime = DateTime.now();
    _updateCount = 0;
    _persistCount = 0;

    // Disable animations during resize for immediate feedback
    panelManager.setResizing(true);

    waypointsLog.finest('Resize started - performance monitoring enabled');
  }

  /// Update resize values with immediate visual feedback and debounced persistence
  void updateResize({
    required double screenWidth,
    required double availableWidth,
    required double currentPanelWidth,
    required double dragDelta,
    required bool showSidebar,
  }) {
    if (!_isResizing) return;

    // Direct pixel-based update - no percentage calculations during drag
    final newPanelWidth = (currentPanelWidth - dragDelta).clamp(200.0, 600.0);

    // Skip update if change is too small (reduces unnecessary state updates)
    if (_lastUpdatedWidth != null &&
        (newPanelWidth - _lastUpdatedWidth!).abs() < minUpdateDelta) {
      return;
    }

    // Store pending values - only calculate ratio when needed for persistence
    _pendingPanelWidth = newPanelWidth;
    _lastUpdatedWidth = newPanelWidth;

    // Apply immediate visual update (direct width, no ratio calculations)
    _applyVisualUpdate(newPanelWidth);

    // Debounce only the state persistence (heavier operation)
    _persistTimer?.cancel();
    _persistTimer = Timer(Duration(milliseconds: persistDelayMs), () {
      if (_pendingPanelWidth != null) {
        // No ratio calculation needed - just persist the width
        _persistState(0.0, _pendingPanelWidth!);
      }
    });
  }

  /// Apply immediate visual updates (lightweight) - direct width update
  void _applyVisualUpdate(double panelWidth) {
    try {
      _updateCount++;
      // Direct width update for immediate visual feedback
      panelManager.updatePanelWidth(panelWidth);
      waypointsLog.finest(
        'Visual update applied (count: $_updateCount) - width: $panelWidth',
      );
    } catch (e, stack) {
      waypointsLog.warning('Error applying visual resize update', e, stack);
    }
  }

  /// Persist state to panel manager (heavier operation)
  void _persistState(double unused, double panelWidth) {
    try {
      _persistCount++;
      // Final width is already set by visual update, just log for monitoring
      // Could add additional persistence logic here (e.g., save to file, analytics, etc.)
      waypointsLog.finest(
        'Resize state persist trigger (count: $_persistCount): width=$panelWidth',
      );
    } catch (e, stack) {
      waypointsLog.warning('Error in resize state persistence', e, stack);
    }
  }

  /// End resize operation and ensure final state is persisted
  void endResize() {
    _isResizing = false;

    // Cancel any pending updates
    _updateTimer?.cancel();
    _persistTimer?.cancel();

    // Apply final state immediately if needed
    if (_pendingPanelWidth != null) {
      // We'll calculate the ratio in the persist call, no need for context here
      _persistState(
        0.0,
        _pendingPanelWidth!,
      ); // Ratio will be recalculated in _persistState
    }

    // Re-enable animations now that resize is complete
    panelManager.setResizing(false);

    // Performance reporting
    if (_resizeStartTime != null) {
      final duration = DateTime.now().difference(_resizeStartTime!);
      waypointsLog.info(
        'Resize completed - Duration: ${duration.inMilliseconds}ms, '
        'Visual updates: $_updateCount, Persist operations: $_persistCount',
      );
    }

    // Clear pending values
    _pendingPanelWidth = null;
    _resizeStartTime = null;

    waypointsLog.finest('Resize ended');
  }

  /// Get current local panel width (for immediate visual feedback)
  double getCurrentPanelWidth(double fallbackWidth) {
    return _pendingPanelWidth ?? fallbackWidth;
  }

  /// Check if resize is active
  bool get isResizing => _isResizing;

  /// Dispose of resources
  void dispose() {
    _updateTimer?.cancel();
    _persistTimer?.cancel();
  }
}

/// Provider for ResizeController
final resizeControllerProvider = Provider<ResizeController>((ref) {
  final controller = ResizeController(ref: ref);
  ref.onDispose(() => controller.dispose());
  return controller;
});
