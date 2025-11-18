import 'dart:ui' show TextAffinity;

import '../super_editor.dart';
import 'thing_cursor_handler.dart';

/// A specialized TextNodePosition for ThingNodes that includes metadata
/// about the cursor zone and interaction behavior.
///
/// This allows ThingNodes to have positions throughout their entire
/// component area (not just text) while maintaining compatibility with
/// SuperEditor's text editing features.
class ThingNodePosition extends TextNodePosition {
  const ThingNodePosition({
    required super.offset,
    super.affinity = TextAffinity.downstream,
    required this.zone,
  });

  /// The cursor zone this position represents
  final ThingCursorZone zone;

  /// Whether this position represents actual editable text
  bool get isTextEditable => zone == ThingCursorZone.textEditable;

  /// Convention: negative offset indicates non-text position
  bool get isNonTextPosition => offset < 0;

  @override
  bool operator ==(Object other) =>
      other is ThingNodePosition && super == other && zone == other.zone;

  @override
  int get hashCode => Object.hash(super.hashCode, zone);

  @override
  ThingNodePosition copyWith({
    int? offset,
    TextAffinity? affinity,
    ThingCursorZone? zone,
  }) {
    return ThingNodePosition(
      offset: offset ?? this.offset,
      affinity: affinity ?? this.affinity,
      zone: zone ?? this.zone,
    );
  }

  @override
  String toString() =>
      'ThingNodePosition(offset: $offset, affinity: $affinity, zone: $zone)';
}
