import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';
import 'package:follow_the_leader/follow_the_leader.dart';
import 'package:path/path.dart';
import 'package:tempo/src/editor/layers/drag_handles.dart';
import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/infrastructure/_logging.dart';
import 'package:tempo/utils/logging.dart';

import 'package:tempo/src/editor/layers/block_highlighter.dart';

class NodeMetadataPlugin extends SuperEditorPlugin {
  static const metadataKey = 'NodeMetadata.meta';

  NodeMetadataPlugin()
    : _contentTapHandler = switch (defaultTargetPlatform) {
        _ => NodeMetadataContentTapDelegate(),
      } {
    pluginNodeMetadataLog.finest(
      'created `_contentTapHandler` = ${_contentTapHandler.runtimeType}',
    );
    pluginNodeMetadataLog.finest('contentTapHandlers = $contentTapHandlers');
  }

  final Map<String, LeaderLink> _nodeHandleLinks = {};

  late final List<EditReaction> _reactions;
  late final List<EditRequestHandler> _requests;

  @override
  List<DocumentKeyboardAction> get keyboardActions => [];

  @override
  List<ComponentBuilder> get componentBuilders => [];

  @override
  List<ContentTapDelegate> get contentTapHandlers => [_contentTapHandler];
  final NodeMetadataContentTapDelegate _contentTapHandler;

  @override
  List<SingleColumnLayoutStylePhase> get appendedStylePhases => const [];

  @override
  List<SuperEditorLayerBuilder> get documentOverlayBuilders => [
    NodeDragHandleLayerBuilder(nodeHandleLinks: _nodeHandleLinks),
  ];

  @override
  List<SuperEditorLayerBuilder> get documentUnderlayBuilders => [
    NodeLeaderLayerBuilder(nodeHandleLinks: _nodeHandleLinks),
    BlockHighlightBuilder(),
  ];

  @override
  void attach(Editor editor) {
    // editor.context.put(taskKey, )
    _contentTapHandler.editor = editor;

    _requests = [
      // changeTaskStatusRequestHandler,
      // taskIndentRequestHandler,
      // splitExistingTaskRequestHandler,
    ];
    editor.requestHandlers.addAll(_requests);

    _reactions = [
      // UpdateSubTaskIndentAfterTaskDeletionReaction(),
    ];
    // editor.reactionPipeline.addAll(_reactions);
  }

  @override
  void detach(Editor editor) {
    editor.reactionPipeline.removeWhere(
      (reaction) => _reactions.contains(reaction),
    );
    editor.requestHandlers.removeWhere(
      (request) => _requests.contains(request),
    );
    _contentTapHandler.editor = null;
  }
}

class NodeMetadataContentTapDelegate extends ContentTapDelegate {
  NodeMetadataContentTapDelegate();

  Editor? editor;
  String? currentNodeID;

  @override
  MouseCursor? mouseCursorForContentHover(DocumentPosition hoverPosition) {
    final layout =
        editor!.context
            .find<DocumentLayoutEditable>(Editor.layoutKey)
            .documentLayout;
    final component = layout.getComponentByNodeId(hoverPosition.nodeId);
    final renderBox = component?.context.findRenderObject() as RenderBox;
    pluginNodeMetadataLog.finest(renderBox.size);
    if (currentNodeID != hoverPosition.nodeId) {
      // currentNodeID = hoverPosition.nodeId;
    }
    return null;
  }

  @override
  TapHandlingInstruction onTap(DocumentTapDetails details) {
    return TapHandlingInstruction.continueHandling;
  }

  @override
  TapHandlingInstruction onDoubleTap(DocumentTapDetails details) {
    return TapHandlingInstruction.continueHandling;
  }

  @override
  TapHandlingInstruction onPanStart(DocumentTapDetails details) {
    return TapHandlingInstruction.continueHandling;
  }

  @override
  TapHandlingInstruction onPanUpdate(DocumentTapDetails details) {
    return TapHandlingInstruction.continueHandling;
  }

  @override
  TapHandlingInstruction onPanEnd(DocumentTapDetails details) {
    return TapHandlingInstruction.continueHandling;
  }
}
