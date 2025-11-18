import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'text_styles.dart';

/// Riverpod provider for the unified Waypoints stylesheet
/// This provides all styling configuration including responsive context
final stylesheetProvider = Provider<WaypointsStylesheet>((ref) {
  throw UnimplementedError(
    'waypointsStylesheetProvider must be overridden in widget tree',
  );
});

/// Responsive breakpoints for different screen sizes
enum WaypointsBreakpoint {
  /// Phone: < 600px
  mobile(maxWidth: 600),

  /// Tablet: 600px - 1024px
  tablet(maxWidth: 1024),

  /// Desktop: > 1024px
  desktop(maxWidth: double.infinity);

  const WaypointsBreakpoint({required this.maxWidth});
  final double maxWidth;

  /// Get breakpoint from screen width
  static WaypointsBreakpoint fromWidth(double width) {
    if (width < mobile.maxWidth) return mobile;
    if (width < tablet.maxWidth) return tablet;
    return desktop;
  }
}

/// Style context that provides responsive information and device characteristics
class WaypointsStyleContext {
  const WaypointsStyleContext({
    required this.screenSize,
    required this.breakpoint,
    required this.devicePixelRatio,
  });

  final Size screenSize;
  final WaypointsBreakpoint breakpoint;
  final double devicePixelRatio;

  /// Convenience getters for breakpoint checks
  bool get isMobile => breakpoint == WaypointsBreakpoint.mobile;
  bool get isTablet => breakpoint == WaypointsBreakpoint.tablet;
  bool get isDesktop => breakpoint == WaypointsBreakpoint.desktop;

  /// Screen width
  double get width => screenSize.width;

  /// Screen height
  double get height => screenSize.height;

  /// Is this a high-DPI display?
  bool get isHighDPI => devicePixelRatio > 2.0;

  /// Get style context from BuildContext
  static WaypointsStyleContext fromContext(BuildContext context) {
    final mediaQuery = MediaQuery.of(context);
    final screenSize = mediaQuery.size;
    final breakpoint = WaypointsBreakpoint.fromWidth(screenSize.width);

    return WaypointsStyleContext(
      screenSize: screenSize,
      breakpoint: breakpoint,
      devicePixelRatio: mediaQuery.devicePixelRatio,
    );
  }
}

/// Unified styling system for Waypoints
///
/// Similar to Flutter's Theme, this provides a single source of truth
/// for all styling decisions across SuperEditor and Flutter widgets.
class WaypointsStylesheet {
  const WaypointsStylesheet({
    required this.textStyles,
    required this.spacing,
    required this.colors,
    required this.context,
  });

  /// Text styles for all text in the app
  final WaypointsTextStyleSet textStyles;

  /// Spacing and padding values (responsive based on context)
  final WaypointsSpacing spacing;

  /// Color extensions beyond Material theme
  final WaypointsColors colors;

  /// Style context with breakpoint and device information
  final WaypointsStyleContext context;

  // Convenience getters for responsive checks (delegates to context)
  bool get isMobile => context.isMobile;
  bool get isTablet => context.isTablet;
  bool get isDesktop => context.isDesktop;
  WaypointsBreakpoint get breakpoint => context.breakpoint;
  double get screenWidth => context.width;
  double get screenHeight => context.height;

  /// Get stylesheet from Riverpod provider (preferred)
  static WaypointsStylesheet of(WidgetRef ref) {
    return ref.read(stylesheetProvider);
  }

  /// Get stylesheet from BuildContext (for functions/static methods)
  static WaypointsStylesheet fromContext(BuildContext context) {
    return _createFromContext(context);
  }

  /// Create stylesheet from BuildContext with theme integration
  static WaypointsStylesheet _createFromContext(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final styleContext = WaypointsStyleContext.fromContext(context);

    return WaypointsStylesheet(
      textStyles: WaypointsTextStyles.fromContext(context),
      spacing: WaypointsSpacing.responsive(styleContext),
      colors: WaypointsColors._fromColorScheme(colorScheme),
      context: styleContext,
    );
  }
}

/// Spacing and padding configuration
class WaypointsSpacing {
  /// Create responsive spacing based on style context
  factory WaypointsSpacing.responsive(WaypointsStyleContext context) {
    return switch (context.breakpoint) {
      WaypointsBreakpoint.mobile => mobile,
      WaypointsBreakpoint.tablet => tablet,
      WaypointsBreakpoint.desktop => desktop,
    };
  }
  const WaypointsSpacing({
    required this.documentPadding,
    required this.contentHorizontal,
    required this.maxContentWidth,
    required this.paragraphSpacing,
    required this.sectionSpacing,
    required this.chipPadding,
  });

  /// Padding around the entire document
  final EdgeInsets documentPadding;

  /// Horizontal padding for content (matches SuperEditor hover highlights)
  final double contentHorizontal;

  /// Maximum width for content
  final double maxContentWidth;

  /// Spacing between paragraphs
  final double paragraphSpacing;

  /// Spacing between sections
  final double sectionSpacing;

  /// Internal padding for chip components
  final EdgeInsets chipPadding;

  /// Mobile spacing (< 600px)
  static const mobile = WaypointsSpacing(
    documentPadding: EdgeInsets.symmetric(horizontal: 0.0, vertical: 16.0),
    contentHorizontal: 0.0,
    maxContentWidth: double.infinity,
    paragraphSpacing: 16.0,
    sectionSpacing: 32.0,
    chipPadding: EdgeInsets.symmetric(horizontal: 6.0, vertical: 3.0),
  );

  /// Tablet spacing (600px - 1024px)
  static const tablet = WaypointsSpacing(
    documentPadding: EdgeInsets.symmetric(horizontal: 16.0, vertical: 20.0),
    contentHorizontal: 16.0,
    maxContentWidth: 800.0,
    paragraphSpacing: 20.0,
    sectionSpacing: 40.0,
    chipPadding: EdgeInsets.symmetric(horizontal: 7.0, vertical: 3.5),
  );

  /// Desktop spacing (> 1024px)
  static const desktop = WaypointsSpacing(
    documentPadding: EdgeInsets.symmetric(horizontal: 24.0, vertical: 24.0),
    contentHorizontal: 24.0,
    maxContentWidth: 800.0,
    paragraphSpacing: 24.0,
    sectionSpacing: 48.0,
    chipPadding: EdgeInsets.symmetric(horizontal: 8.0, vertical: 4.0),
  );
}

/// Color extensions for Waypoints-specific needs
class WaypointsColors {
  /// Create colors from Material ColorScheme
  factory WaypointsColors._fromColorScheme(ColorScheme scheme) {
    return WaypointsColors(
      taskOverdue: scheme.error,
      taskToday: scheme.tertiary,
      taskHighPriority: scheme.primary,
      projectActive: scheme.primary,
      projectArchived: scheme.onSurfaceVariant.withOpacity(0.6),
      editorHighlight: scheme.primary.withOpacity(0.1),
      editorSelection: scheme.primary.withOpacity(0.3),
    );
  }
  const WaypointsColors({
    required this.taskOverdue,
    required this.taskToday,
    required this.taskHighPriority,
    required this.projectActive,
    required this.projectArchived,
    required this.editorHighlight,
    required this.editorSelection,
  });

  final Color taskOverdue;
  final Color taskToday;
  final Color taskHighPriority;
  final Color projectActive;
  final Color projectArchived;
  final Color editorHighlight;
  final Color editorSelection;
}
