import 'dart:convert';

import 'package:objectbox/objectbox.dart';
import 'package:super_editor/super_editor.dart';

import '../../objectbox/attributed_text_mixin.dart';
import '../thing/interface.dart';
import '../thing/thing.entity.dart';

/// Generic content block that can store any type of SuperEditor node.
/// Follows the Thing architecture for polymorphic relationships and UUID resolution.
@Entity()
class BlockEntity
    with AttributedTextEntity, ThingBehavior
    implements BaseThing {
  BlockEntity({
    required this.uuid,
    required this.nodeType,
    AttributedText? content,
    DateTime? createDate,
    DateTime? modifyDate,
    this.metadataJson,
    this.archived = false,
    this.deleted = false,
  }) : createDate = createDate ?? DateTime.now(),
       modifyDate = modifyDate ?? DateTime.now() {
    this.content = content ?? AttributedText('');
  }

  factory BlockEntity.fromNode(DocumentNode node) {
    if (node is! TextNode) {
      throw ArgumentError(
        'node must inherit from `TextNode`! Instead found ${node.runtimeType}.',
      );
    }
    return BlockEntity(
      uuid: node.id,
      nodeType: node.getMetadataValue('blockType'),
      metadataJson: json.encode(node.copyMetadata()),
      content: node.text,
    );
  }

  static const key = 'block';

  @Id()
  @override
  int id = 0;

  @override
  String uuid;

  /// The type of SuperEditor node (e.g., 'paragraph', 'header', 'list_item')
  String nodeType;

  /// JSON representation of the AttributedText content
  String contentJson = '';

  /// User-defined metadata as JSON for extensibility
  String? metadataJson;

  @Property(type: PropertyType.dateNano)
  @override
  DateTime createDate;

  @Property(type: PropertyType.dateNano)
  @override
  DateTime modifyDate;

  @override
  bool archived;

  @override
  bool deleted;

  /// Direct reference to the ThingEntity registration
  @override
  final ToOne<ThingEntity> thing = ToOne<ThingEntity>();

  /// History snapshots for this block
  final history = ToMany<BlockSnapshotEntity>();

  /// AttributedText content getter/setter using the mixin
  @Transient()
  AttributedText get content => getAttributedText(contentJson);
  set content(AttributedText value) =>
      setAttributedText(value, (json) => contentJson = json);

  @override
  String get type => key;

  /// Create a new snapshot for history tracking
  BlockSnapshotEntity snapshot() {
    return BlockSnapshotEntity(
      type: nodeType,
      contentJson: contentJson,
      metadataJson: metadataJson,
    );
  }

  BlockEntity copyWith({
    String? uuid,
    String? nodeType,
    AttributedText? content,
    String? parentUuid,
    String? documentUuid,
    int? orderIndex,
    DateTime? createDate,
    String? metadataJson,
    bool? archived,
    bool? deleted,
  }) {
    final block = BlockEntity(
      uuid: uuid ?? this.uuid,
      nodeType: nodeType ?? this.nodeType,
      createDate: createDate ?? this.createDate,
      modifyDate: DateTime.now(),
      metadataJson: metadataJson ?? this.metadataJson,
      archived: archived ?? this.archived,
      deleted: deleted ?? this.deleted,
    );
    block.id = id;
    block.content = content ?? this.content;
    return block;
  }

  @override
  String toString() {
    return 'BlockEntity{id: $id, uuid: $uuid, nodeType: $nodeType, content: "${content.toPlainText()}"}';
  }

  @override
  String getSearchableText() => content.toPlainText();
}

/// Snapshot entity for block history tracking
@Entity()
class BlockSnapshotEntity with AttributedTextEntity {
  BlockSnapshotEntity({
    required this.type,
    required this.contentJson,
    this.metadataJson,
  });

  @Id()
  int id = 0;

  /// Reference to the parent block
  final block = ToOne<BlockEntity>();

  @Property(type: PropertyType.dateNano)
  DateTime timestamp = DateTime.now();

  /// Snapshot data
  String type;
  String contentJson;
  String? metadataJson;

  /// AttributedText content getter/setter using the mixin
  @Transient()
  AttributedText? get content {
    return getAttributedText(contentJson);
  }

  set content(AttributedText? value) {
    if (value == null) return;
    setAttributedText(value, (json) => contentJson = json);
  }
}
