import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../infrastructure/logging.dart';
import '../../../styling/text_styles.dart';

/// Public interface for controlling the app bar's scroll state
abstract class PanelAppBarController {
  void updateScrollPosition(double pixels, [double? headerHeight]);
}

/// Sliver version of PanelAppBar that integrates with CustomScrollView
/// Reuses the core animation logic from the original PanelAppBar
class PanelAppBar extends ConsumerStatefulWidget {
  const PanelAppBar({
    super.key,
    required this.title,
    this.actions = const [],
    this.padding = const EdgeInsets.all(8.0),
    this.scrollThreshold = 100.0, // When to show title animation
  });

  final String title;
  final List<Widget> actions;
  final EdgeInsetsGeometry padding;
  final double scrollThreshold; // Reusable threshold from original

  @override
  ConsumerState<PanelAppBar> createState() => PanelAppBarSliverState();

  /// Compute the app bar height using the same logic as the original
  static double computeHeight(BuildContext context) {
    final textStyles = WaypointsTextStyles.fromContext(context);
    final fontSize = textStyles.bodyMedium.fontSize!;
    final appBarHeight = fontSize * 4;
    return appBarHeight + 1;
  }
}

class PanelAppBarSliverState extends ConsumerState<PanelAppBar>
    implements PanelAppBarController {
  // 📋 REUSED: Same animation state logic as original PanelAppBar
  double _titleOpacity = 0.0;
  double _borderOpacity = 0.0;

  // 🎯 NEW: Dynamic scroll-based transition using header height
  @override
  void updateScrollPosition(double pixels, [double? headerHeight]) {
    final threshold = headerHeight ?? widget.scrollThreshold;

    // 🎯 NEW: Calculate gradual opacity based on scroll progress through header
    final scrollProgress = (pixels / threshold).clamp(0.0, 1.0);

    waypointsPanelLog.finest(
      'Scroll: ${pixels}px / ${threshold}px = ${scrollProgress} opacity',
    );

    // 🎯 NEW: Update opacity values for smooth transition
    final newTitleOpacity = scrollProgress;
    final newBorderOpacity = scrollProgress;

    if (newTitleOpacity != _titleOpacity ||
        newBorderOpacity != _borderOpacity) {
      setState(() {
        _titleOpacity = newTitleOpacity;
        _borderOpacity = newBorderOpacity;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final textStyles = WaypointsTextStyles.of(ref);
    // 🎯 NEW: Return SliverAppBar without NotificationListener
    // The listener needs to be at the CustomScrollView level, not here
    return SliverAppBar(
      pinned: true, // 🎯 NEW: Sticky behavior for sliver
      backgroundColor: ColorScheme.of(context).surfaceContainer,
      elevation: 0,
      scrolledUnderElevation: 0,

      // 🎯 NEW: Smooth title transition based on scroll progress
      title: Transform.translate(
        offset: Offset(
          0,
          (1.0 - _titleOpacity) * 20,
        ), // Slide up as it fades in
        child: Opacity(opacity: _titleOpacity, child: Text(widget.title)),
      ),

      // 📋 REUSED: Same actions and padding from original
      actions: widget.actions,
      actionsPadding: widget.actions.isEmpty
          ? EdgeInsets.all(0)
          : EdgeInsets.symmetric(horizontal: textStyles.bodyLarge.fontSize!),

      // 🎯 NEW: Use bottom property for the animated border (SliverAppBar-compatible)
      bottom: PreferredSize(
        preferredSize: const Size.fromHeight(1.0),
        child: _AnimatedBorder(
          opacity: _borderOpacity, // 🎯 NEW: Use dynamic opacity
        ),
      ),
    );
  }
}

/// 📋 REUSED: Same animated border logic from original PanelAppBar
/// 🎯 FIXED: Removed Positioned wrapper for SliverAppBar compatibility
/// 🎯 NEW: Uses dynamic opacity for smooth transitions
class _AnimatedBorder extends StatelessWidget {
  const _AnimatedBorder({required this.opacity});

  final double opacity;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 1,
      color: Color.lerp(
        ColorScheme.of(context).surfaceContainer,
        ColorScheme.of(context).outlineVariant,
        opacity, // 🎯 NEW: Smoothly interpolate between colors based on scroll
      ),
    );
  }
}
