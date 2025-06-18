import 'package:flutter/rendering.dart';

import 'super_editor.dart';

SuperEditorAddEmptyNodeTapHandler
superEditorAddEmptyNodeTapHandlerFactory<N extends TextNode>(
  SuperEditorContext editContext,
) => SuperEditorAddEmptyNodeTapHandler<N>(editContext: editContext);

/// A [ContentTapDelegate] that adds an empty paragraph at the end of the document
/// when the user taps below the last node in the document.
///
/// Does nothing if the last node is a [TextNode].
class SuperEditorAddEmptyNodeTapHandler<NodeType extends TextNode>
    extends ContentTapDelegate {
  SuperEditorAddEmptyNodeTapHandler({required this.editContext});

  final SuperEditorContext editContext;

  @override
  TapHandlingInstruction onTap(DocumentTapDetails details) {
    final tapPosition = details.documentLayout
        .getDocumentPositionNearestToOffset(details.layoutOffset);
    if (tapPosition == null) {
      return TapHandlingInstruction.continueHandling;
    }

    final editor = editContext.editor;
    final document = editContext.document;

    final node = document.getNodeById(tapPosition.nodeId)!;
    if (node is TextNode) {
      return TapHandlingInstruction.continueHandling;
    }

    if (!_isTapBelowLastNode(
      nodeId: tapPosition.nodeId,
      globalOffset: details.globalOffset,
    )) {
      return TapHandlingInstruction.continueHandling;
    }

    // The user tapped below a non-text node. Add a new paragraph
    // to the end of the document and place the caret there.
    final newNodeId = Editor.createNodeId();
    editor.execute([
      InsertNodeAfterNodeRequest(
        existingNodeId: node.id,
        newNode: TextNode(id: newNodeId, text: AttributedText()) as NodeType,
      ),
      ChangeSelectionRequest(
        DocumentSelection.collapsed(
          position: DocumentPosition(
            nodeId: newNodeId,
            nodePosition: const TextNodePosition(offset: 0),
          ),
        ),
        SelectionChangeType.insertContent,
        SelectionReason.userInteraction,
      ),
      const ClearComposingRegionRequest(),
    ]);

    return TapHandlingInstruction.halt;
  }

  bool _isTapBelowLastNode({
    required String nodeId,
    required Offset globalOffset,
  }) {
    final documentLayout = editContext.documentLayout;
    final document = editContext.document;

    final tappedComponent = documentLayout.getComponentByNodeId(nodeId)!;
    final componentBox =
        tappedComponent.context.findRenderObject() as RenderBox;
    final localPosition = componentBox.globalToLocal(globalOffset);
    final node = document.getNodeById(nodeId);

    return (node == document.lastOrNull) &&
        (localPosition.dy > componentBox.size.height);
  }
}
