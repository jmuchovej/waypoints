import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:super_editor/super_editor.dart';

import '../thing_factories.dart';
import 'block.entity.dart';
import 'block_repository.dart';

/// Factory to convert BlockEntity to appropriate SuperEditor DocumentNode
class BlockNodeFactory
    extends ThingNodeFactory<BlockEntity, BlockRepository, DocumentNode> {
  BlockNodeFactory(Ref ref) : super(ref, blockRepositoryProvider, fromBlock);

  @override
  String get type => BlockEntity.key;

  /// Convert a BlockEntity directly to the appropriate SuperEditor node type
  static DocumentNode fromBlock(BlockEntity block) {
    // Parse any additional metadata from the block
    Map<String, dynamic>? metadata;
    if (block.metadataJson != null) {
      try {
        metadata = json.decode(block.metadataJson!) as Map<String, dynamic>;
      } catch (e) {
        // Invalid JSON, ignore
      }
    }

    // Create the appropriate node type based on nodeType
    switch (block.nodeType) {
      case 'paragraph':
        return ParagraphNode(
          id: block.uuid,
          text: block.content,
          metadata: metadata,
        );

      case 'header1':
        return ParagraphNode(
          id: block.uuid,
          text: block.content,
          metadata: {...?metadata, 'blockType': header1Attribution},
        );

      case 'header2':
        return ParagraphNode(
          id: block.uuid,
          text: block.content,
          metadata: {...?metadata, 'blockType': header2Attribution},
        );

      case 'header3':
        return ParagraphNode(
          id: block.uuid,
          text: block.content,
          metadata: {...?metadata, 'blockType': header3Attribution},
        );

      case 'header4':
        return ParagraphNode(
          id: block.uuid,
          text: block.content,
          metadata: {...?metadata, 'blockType': header4Attribution},
        );

      case 'header5':
        return ParagraphNode(
          id: block.uuid,
          text: block.content,
          metadata: {...?metadata, 'blockType': header5Attribution},
        );

      case 'header6':
        return ParagraphNode(
          id: block.uuid,
          text: block.content,
          metadata: {...?metadata, 'blockType': header6Attribution},
        );

      case 'blockquote':
        return ParagraphNode(
          id: block.uuid,
          text: block.content,
          metadata: {...?metadata, 'blockType': blockquoteAttribution},
        );

      case 'list_item':
        return ListItemNode(
          id: block.uuid,
          itemType: ListItemType.unordered,
          text: block.content,
          metadata: metadata,
        );

      case 'ordered_list_item':
        return ListItemNode(
          id: block.uuid,
          itemType: ListItemType.ordered,
          text: block.content,
          metadata: metadata,
        );

      case 'horizontal_rule':
        return HorizontalRuleNode(id: block.uuid, metadata: metadata);

      case 'image':
        // Extract image URL from metadata or content
        final imageUrl =
            metadata?['url'] as String? ?? block.content.toPlainText();
        final altText = metadata?['altText'] as String?;
        return ImageNode(
          id: block.uuid,
          imageUrl: imageUrl,
          altText: altText ?? '',
          metadata: metadata,
        );

      default:
        // Default to paragraph for unknown types
        return ParagraphNode(
          id: block.uuid,
          text: block.content,
          metadata: metadata,
        );
    }
  }
}
