import 'package:meta/meta.dart';
import '../editor/super_editor.dart';
import '../editor/attributed_text.dart';

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
///     value,
///     (json) => _titleJson = json,
///     (plain) => titlePlainText = plain,
///   );
/// }
/// ```
mixin AttributedTextEntity {
  /// Helper to get AttributedText from stored JSON
  @protected
  AttributedText getAttributedText(String jsonString) {
    return AttributedTextJson.fromJsonString(jsonString);
  }

  /// Helper to set AttributedText and update both JSON and plain text
  @protected
  void setAttributedText(
    AttributedText value,
    void Function(String json) setJson,
  ) {
    setJson(value.toJsonString());
  }
}
