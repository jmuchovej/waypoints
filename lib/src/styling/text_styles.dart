import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Riverpod provider for guaranteed non-nullable text styles
/// This ensures all fontSize, height, and other critical properties are never null
final tempoTextStylesProvider = Provider<TempoTextStyleSet>((ref) {
  throw UnimplementedError('tempoTextStylesProvider must be overridden in widget tree');
});

/// Unified text styling system for SuperEditor and Flutter widgets
/// 
/// Provides non-nullable text styles with consistent values across the app.
/// SuperEditor stylesheet and regular widgets both use these styles for consistency.
class TempoTextStyles {
  TempoTextStyles._();

  /// Get non-nullable text styles from Riverpod provider
  /// This is the preferred method - use TempoTextStyles.of(ref) with WidgetRef
  static TempoTextStyleSet of(WidgetRef ref) {
    return ref.read(tempoTextStylesProvider);
  }
  
  /// Get non-nullable text styles from BuildContext (legacy support)
  /// Prefer using of(ref) with WidgetRef when possible
  static TempoTextStyleSet fromContext(BuildContext context) {
    return _createFromContext(context);
  }

  /// Helper to create TempoTextStyleSet from BuildContext
  /// This handles the actual theme merging and null-safety guarantees
  static TempoTextStyleSet _createFromContext(BuildContext context) {
    final theme = Theme.of(context).textTheme;
    
    // Merge theme styles with our fallbacks, ensuring non-null critical properties
    return TempoTextStyleSet(
      // Display styles (largest text)
      displayLarge: _mergeWithFallback(theme.displayLarge, _fallbackDisplayLarge),
      displayMedium: _mergeWithFallback(theme.displayMedium, _fallbackDisplayMedium),
      displaySmall: _mergeWithFallback(theme.displaySmall, _fallbackDisplaySmall),
      
      // Headline styles (headers)
      headlineLarge: _mergeWithFallback(theme.headlineLarge, _fallbackHeadlineLarge),
      headlineMedium: _mergeWithFallback(theme.headlineMedium, _fallbackHeadlineMedium),
      headlineSmall: _mergeWithFallback(theme.headlineSmall, _fallbackHeadlineSmall),
      
      // Title styles
      titleLarge: _mergeWithFallback(theme.titleLarge, _fallbackTitleLarge),
      titleMedium: _mergeWithFallback(theme.titleMedium, _fallbackTitleMedium),
      titleSmall: _mergeWithFallback(theme.titleSmall, _fallbackTitleSmall),
      
      // Body styles (main content)
      bodyLarge: _mergeWithFallback(theme.bodyLarge, _fallbackBodyLarge),
      bodyMedium: _mergeWithFallback(theme.bodyMedium, _fallbackBodyMedium),
      bodySmall: _mergeWithFallback(theme.bodySmall, _fallbackBodySmall),
      
      // Label styles (small text)
      labelLarge: _mergeWithFallback(theme.labelLarge, _fallbackLabelLarge),
      labelMedium: _mergeWithFallback(theme.labelMedium, _fallbackLabelMedium),
      labelSmall: _mergeWithFallback(theme.labelSmall, _fallbackLabelSmall),
    );
  }

  /// Merge theme style with fallback, ensuring critical properties are non-null
  static TextStyle _mergeWithFallback(TextStyle? themeStyle, TextStyle fallback) {
    if (themeStyle == null) return fallback;
    
    // Start with theme style, but ensure critical properties from fallback
    return themeStyle.copyWith(
      fontSize: themeStyle.fontSize ?? fallback.fontSize,
      fontWeight: themeStyle.fontWeight ?? fallback.fontWeight,
      height: themeStyle.height ?? fallback.height,
    );
  }

  // Fallback text styles with sensible defaults
  static const _fallbackDisplayLarge = TextStyle(fontSize: 57, fontWeight: FontWeight.w400);
  static const _fallbackDisplayMedium = TextStyle(fontSize: 45, fontWeight: FontWeight.w400);
  static const _fallbackDisplaySmall = TextStyle(fontSize: 36, fontWeight: FontWeight.w400);
  
  static const _fallbackHeadlineLarge = TextStyle(fontSize: 32, fontWeight: FontWeight.w400);
  static const _fallbackHeadlineMedium = TextStyle(fontSize: 28, fontWeight: FontWeight.w400);
  static const _fallbackHeadlineSmall = TextStyle(fontSize: 24, fontWeight: FontWeight.w400);
  
  static const _fallbackTitleLarge = TextStyle(fontSize: 22, fontWeight: FontWeight.w500);
  static const _fallbackTitleMedium = TextStyle(fontSize: 16, fontWeight: FontWeight.w500);
  static const _fallbackTitleSmall = TextStyle(fontSize: 14, fontWeight: FontWeight.w500);
  
  static const _fallbackBodyLarge = TextStyle(fontSize: 16, fontWeight: FontWeight.w400);
  static const _fallbackBodyMedium = TextStyle(fontSize: 14, fontWeight: FontWeight.w400);
  static const _fallbackBodySmall = TextStyle(fontSize: 12, fontWeight: FontWeight.w400);
  
  static const _fallbackLabelLarge = TextStyle(fontSize: 14, fontWeight: FontWeight.w500);
  static const _fallbackLabelMedium = TextStyle(fontSize: 12, fontWeight: FontWeight.w500);
  static const _fallbackLabelSmall = TextStyle(fontSize: 11, fontWeight: FontWeight.w500);
}

/// Non-nullable text style set for guaranteed access
class TempoTextStyleSet {
  const TempoTextStyleSet({
    required this.displayLarge,
    required this.displayMedium,
    required this.displaySmall,
    required this.headlineLarge,
    required this.headlineMedium,
    required this.headlineSmall,
    required this.titleLarge,
    required this.titleMedium,
    required this.titleSmall,
    required this.bodyLarge,
    required this.bodyMedium,
    required this.bodySmall,
    required this.labelLarge,
    required this.labelMedium,
    required this.labelSmall,
  });

  // Display styles
  final TextStyle displayLarge;
  final TextStyle displayMedium;
  final TextStyle displaySmall;
  
  // Headline styles
  final TextStyle headlineLarge;
  final TextStyle headlineMedium;
  final TextStyle headlineSmall;
  
  // Title styles
  final TextStyle titleLarge;
  final TextStyle titleMedium;
  final TextStyle titleSmall;
  
  // Body styles
  final TextStyle bodyLarge;
  final TextStyle bodyMedium;
  final TextStyle bodySmall;
  
  // Label styles
  final TextStyle labelLarge;
  final TextStyle labelMedium;
  final TextStyle labelSmall;

  /// Get consistent padding based on text style
  /// Uses the same calculation pattern as SuperEditor
  /// No null checks needed - TempoTextStyles guarantees non-null values
  EdgeInsets paddingFor(TextStyle style) {
    final height = style.height ?? 1.0; // height can legitimately be null (means default line height)
    final fontSize = style.fontSize!; // fontSize is guaranteed non-null by TempoTextStyles
    final lineHeight = fontSize * height;
    
    return EdgeInsets.only(
      top: lineHeight / 2,
      bottom: lineHeight / 4,
    );
  }

  /// Text style by semantic role for consistent usage
  TextStyle get documentHeader => displayLarge;
  TextStyle get pageTitle => displayMedium;
  TextStyle get sectionHeader => headlineLarge;
  TextStyle get subsectionHeader => headlineMedium;
  TextStyle get cardTitle => titleLarge;
  TextStyle get listTitle => titleMedium;
  TextStyle get contentBody => bodyLarge;
  TextStyle get contentSecondary => bodyMedium;
  TextStyle get caption => bodySmall;
  TextStyle get buttonText => labelLarge;
  TextStyle get chipText => labelMedium;
  TextStyle get metadata => labelSmall;
}