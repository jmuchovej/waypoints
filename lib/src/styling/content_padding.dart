import 'package:flutter/material.dart';

import '../infrastructure/platform.dart';

/// Shared content area padding constants for visual alignment across the app
/// 
/// Ensures external components align with SuperEditor's content area and hover highlights.
/// SuperEditor uses these same values in its _HighlightWrapper for consistent visual flow.
class ContentPadding {
  ContentPadding._();

  /// Horizontal padding for content area
  /// - Desktop: 10px to align with SuperEditor hover highlights  
  /// - Mobile: 0px for full-width content
  static double get horizontal => CurrentPlatform.isMobile ? 0.0 : 10.0;

  /// EdgeInsets for horizontal content padding
  static EdgeInsets get horizontalInsets => EdgeInsets.symmetric(horizontal: horizontal);

  /// EdgeInsets for full content area padding (horizontal + optional vertical)
  static EdgeInsets contentInsets({double vertical = 0.0}) => EdgeInsets.symmetric(
    horizontal: horizontal,
    vertical: vertical,
  );

  /// Maximum content width constraint (matches SuperEditor's stylesheet)
  static const double maxContentWidth = 800.0;

  /// Combined content container: max width + horizontal padding
  static Widget contentContainer({required Widget child}) => Center(
    child: ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: maxContentWidth),
      child: Padding(
        padding: horizontalInsets,
        child: child,
      ),
    ),
  );
}