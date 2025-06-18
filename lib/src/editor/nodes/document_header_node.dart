import '../editor.dart';
import '../../infrastructure/logging.dart';
import '../stylesheet.dart' show documentHeaderAttribution;

/// A document header node that displays metadata from the entity driving the document.
///
/// This node is special in that it:
/// - Cannot be dragged/reordered
/// - Cannot be deleted
/// - Is populated with metadata from external entities
/// - Does not participate in normal text editing
class DocumentHeaderNode extends BlockNode {
  DocumentHeaderNode({
    required this.id,
    required this.title,
    this.subtitle,
    this.icon,
    super.metadata,
  }) : super() {
    initAddToMetadata({"blockType": documentHeaderAttribution});

    // Log when DocumentHeaderNode is created
    tempoLog.info(
      '📄 DocumentHeaderNode created: id=$id, title="$title", '
      'blockType=${getMetadataValue("blockType")}',
    );
  }

  @override
  final String id;

  /// The primary title text
  final String title;

  /// Optional subtitle text
  final String? subtitle;

  /// Optional icon identifier
  final String? icon;

  @override
  int get hashCode => Object.hash(id, title, subtitle, icon, metadata);

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! DocumentHeaderNode) return false;

    return id == other.id &&
        title == other.title &&
        subtitle == other.subtitle &&
        icon == other.icon &&
        metadata == other.metadata;
  }

  @override
  DocumentNode copyAndReplaceMetadata(Map<String, dynamic> newMetadata) {
    return copyWith(metadata: newMetadata);
  }

  // BlockNode provides default implementations for position methods

  @override
  String? copyContent(NodeSelection selection) => title;

  DocumentHeaderNode copyWith({
    String? id,
    String? title,
    String? subtitle,
    String? icon,
    Map<String, dynamic>? metadata,
  }) {
    return DocumentHeaderNode(
      id: id ?? this.id,
      title: title ?? this.title,
      subtitle: subtitle ?? this.subtitle,
      icon: icon ?? this.icon,
      metadata: metadata ?? this.metadata,
    );
  }

  @override
  DocumentNode copyWithAddedMetadata(Map<String, dynamic> newProperties) {
    final newMetadata = Map<String, dynamic>.from(metadata);
    newMetadata.addAll(newProperties);
    return copyWith(metadata: newMetadata);
  }

  @override
  bool hasEquivalentContent(DocumentNode other) {
    if (other is! DocumentHeaderNode) {
      return false;
    }

    return title == other.title &&
        subtitle == other.subtitle &&
        icon == other.icon;
  }
}

/// Position class for DocumentHeaderNode
class DocumentHeaderPosition extends NodePosition {
  DocumentHeaderPosition();

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  bool operator ==(Object other) => other is DocumentHeaderPosition;

  @override
  bool isEquivalentTo(NodePosition other) => other is DocumentHeaderPosition;
}

/// Selection class for DocumentHeaderNode
class DocumentHeaderSelection extends NodeSelection {
  DocumentHeaderSelection();

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  bool operator ==(Object other) => other is DocumentHeaderSelection;
}
