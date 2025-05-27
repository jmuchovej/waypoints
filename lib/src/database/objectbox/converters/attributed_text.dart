import 'dart:convert';
import 'package:meta/meta.dart';
import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/editor/attributed_text.dart';

/// Helper class for storing AttributedText in ObjectBox
///
/// Since ObjectBox doesn't support custom converters directly,
/// this class provides utilities to handle AttributedText storage
/// with both JSON (for full fidelity) and plain text (for querying).
class ObjectBoxAttributedText {
  /// Convert AttributedText to JSON string for storage
  static String toJson(AttributedText text) {
    return jsonEncode(text.toJson());
  }

  /// Convert JSON string back to AttributedText
  static AttributedText fromJson(String json) {
    if (json.isEmpty) return AttributedText();
    try {
      final data = jsonDecode(json) as Map<String, dynamic>;
      return AttributedTextJSON.fromJson(data);
    } catch (e) {
      // If JSON is invalid, return empty AttributedText
      return AttributedText();
    }
  }

  /// Extract plain text for querying
  static String toPlainText(AttributedText text) {
    return text.toPlainText();
  }
}

/// Mixin to add AttributedText support to ObjectBox entities
///
/// Usage:
/// ```dart
/// @Entity()
/// class MyEntity with AttributedTextEntity {
///   @Id()
///   int id = 0;
///
///   // Store the JSON representation
///   String _titleJson = '';
///
///   // Store plain text for querying
///   @Index()
///   String titlePlainText = '';
///
///   // Use the mixin's helper methods
///   AttributedText get title => getAttributedText(_titleJson, titlePlainText);
///   set title(AttributedText value) => setAttributedText(
///     value,
///     (json) => _titleJson = json,
///     (plain) => titlePlainText = plain,
///   );
/// }
/// ```
mixin AttributedTextEntity {
  /// Helper to get AttributedText from stored JSON
  @protected
  AttributedText getAttributedText(String jsonString, String fallbackPlainText) {
    if (jsonString.isEmpty) return AttributedText(fallbackPlainText);
    return ObjectBoxAttributedText.fromJson(jsonString);
  }

  /// Helper to set AttributedText and update both JSON and plain text
  @protected
  void setAttributedText(
    AttributedText value,
    void Function(String json) setJson,
    void Function(String plainText) setPlainText,
  ) {
    setJson(ObjectBoxAttributedText.toJson(value));
    setPlainText(ObjectBoxAttributedText.toPlainText(value));
  }
}
