import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/infrastructure/_logging.dart';
import 'package:tempo/src/models.dart';

import '_components.dart';
import '_keybinds.dart';
import '_reactions.dart';
import '_requests.dart';

class TaskPlugin extends SuperEditorPlugin {
  static const taskKey = 'TaskPlugin.task';

  TaskPlugin({
    this.onTaskTap,
  })  : _componentBuilder = TaskComponentBuilder(),
        _selectionStyler = TaskSelectionStyler(),
        _contentTapHandler = switch (defaultTargetPlatform) {
          _ => TaskContentTapDelegate(onTaskTap: onTaskTap),
        } {
    pluginTaskLog.finest(
      'created `_componentBuilder` = ${_componentBuilder.runtimeType}',
    );
    pluginTaskLog.finest(
      'created `_selectionStyler` = ${_selectionStyler.runtimeType}',
    );
    pluginTaskLog.finest(
      'created `_contentTapHandler` = ${_contentTapHandler.runtimeType}',
    );
    pluginTaskLog.finest('contentTapHandlers = $contentTapHandlers');
  }

  final void Function(TaskEntity task)? onTaskTap;

  late final List<EditReaction> _reactions;
  late final List<EditRequestHandler> _requests;

  @override
  List<DocumentKeyboardAction> get keyboardActions => [
    // enterToInsertNewTask,
    // tabToIndentTask,
    // shiftTabToUnIndentTask,
    // backspaceToUnIndentTask,
  ];

  @override
  List<ComponentBuilder> get componentBuilders => [_componentBuilder];
  final TaskComponentBuilder _componentBuilder;

  @override
  List<ContentTapDelegate> get contentTapHandlers => [_contentTapHandler];
  final TaskContentTapDelegate _contentTapHandler;

  @override
  List<SingleColumnLayoutStylePhase> get appendedStylePhases => [
    _selectionStyler,
  ];
  final TaskSelectionStyler _selectionStyler;

  @override
  List<SuperEditorLayerBuilder> get documentOverlayBuilders => const [];

  @override
  List<SuperEditorLayerBuilder> get documentUnderlayBuilders => const [];

  @override
  void attach(Editor editor) {
    // editor.context.put(taskKey, )
    _componentBuilder.editor = editor;
    _contentTapHandler.editor = editor;
    _selectionStyler.document = editor.document;
    _selectionStyler.selection = editor.composer.selectionNotifier;
    _selectionStyler.selection!.addListener(_selectionStyler.markDirty);

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
    _componentBuilder.editor = null;
    _selectionStyler.selection!.removeListener(_selectionStyler.markDirty);
    _selectionStyler.selection = null;
    _selectionStyler.document = null;
  }
}

class TaskTapOverlayBuilder extends SuperEditorLayerBuilder {
  @override
  ContentLayerWidget build(
    BuildContext context,
    SuperEditorContext editContext,
  ) {
    return ContentLayerProxyWidget(
      child: Listener(
        behavior: HitTestBehavior.translucent,
        onPointerDown: (PointerDownEvent event) {
          final layout = editContext.documentLayout;
          final position = layout.getDocumentPositionAtOffset(
            event.localPosition,
          );

          if (position != null) {
            editContext.editor.execute([
              ChangeSelectionRequest(
                DocumentSelection.collapsed(position: position),
                SelectionChangeType.placeCaret,
                SelectionReason.userInteraction,
              ),
            ]);
          }
        },
        child: Container(color: Colors.transparent),
      ),
    );
  }
}

class TaskContentTapDelegate extends ContentTapDelegate {
  TaskContentTapDelegate({
    this.onTaskTap,
  });

  Editor? editor;
  final void Function(TaskEntity task)? onTaskTap;

  @override
  MouseCursor? mouseCursorForContentHover(DocumentPosition hoverPosition) {
    if (editor == null) return null;
    final node = editor!.document.getNodeById(hoverPosition.nodeId);
    if (node is TaskNode) {
      return SystemMouseCursors.text;
    }
    return null;
  }

  @override
  TapHandlingInstruction onTap(DocumentTapDetails details) {
    pluginTaskLog.finest('received a single-tap!');
    if (editor == null) {
      pluginTaskLog.finest('¿editor is null?');
      return TapHandlingInstruction.continueHandling;
    }

    final tapPosition = details.documentLayout
        .getDocumentPositionNearestToOffset(details.layoutOffset);
    pluginTaskLog.finest('tapPosition := ${tapPosition}');
    if (tapPosition == null) {
      return TapHandlingInstruction.continueHandling;
    }

    final nodeId = tapPosition.nodeId; // sugar
    final node = editor!.document.getNodeById(nodeId);
    pluginTaskLog.fine("A node was tapped! $nodeId, type: ${node.runtimeType}");
    if (node is! TaskNode) {
      return TapHandlingInstruction.continueHandling;
    }

    pluginTaskLog.fine("  Indeed, a task node!");

    // Navigate to task editor if callback is provided
    if (onTaskTap != null) {
      onTaskTap!(node.task);
    }

    return TapHandlingInstruction.halt;
  }

  @override
  TapHandlingInstruction onDoubleTap(DocumentTapDetails details) {
    pluginTaskLog.fine("A task was double-tapped!");
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

class TaskSelectionStyler implements SingleColumnLayoutStylePhase {
  TaskSelectionStyler();

  Document? document;
  ValueListenable<DocumentSelection?>? selection;

  @override
  set dirtyCallback(VoidCallback? newCallback) {
    // TODO: implement dirtyCallback
  }

  @override
  void dispose() {}

  @override
  void markDirty() {
    // TODO: implement markDirty
  }

  @override
  SingleColumnLayoutViewModel style(
    Document document,
    SingleColumnLayoutViewModel viewModel,
  ) {
    return SingleColumnLayoutViewModel(
      padding: viewModel.padding,
      componentViewModels: [
        for (final prevViewModel in viewModel.componentViewModels) //
          _applyTaskSelection(prevViewModel.copy()),
      ],
    );
  }

  SingleColumnLayoutComponentViewModel _applyTaskSelection(
    SingleColumnLayoutComponentViewModel vm,
  ) {
    final node = document!.getNodeById(vm.nodeId);
    if (node is! TaskNode || vm is! TaskComponentViewModel) {
      return vm;
    }

    final documentSelection = selection!.value;
    if (documentSelection == null) {
      return vm;
    }

    DocumentNodeSelection? nodeSelection;
    late List<DocumentNode> selectedNodes;
    try {
      selectedNodes = document!.getNodesInside(
        documentSelection.base,
        documentSelection.extent,
      );
    } catch (exception) {
      selectedNodes = [];
    }

    nodeSelection = _computeNodeSelection(
      documentSelection,
      selectedNodes,
      node,
    );

    vm.isSelected = nodeSelection != null;

    return vm;
  }

  DocumentNodeSelection? _computeNodeSelection(
    DocumentSelection documentSelection,
    List<DocumentNode> selectedNodes,
    DocumentNode node,
  ) {
    editorStyleLog.finer('_computeNodeSelection(): ${node.id}');
    editorStyleLog.finer(' - base: ${documentSelection.base.nodeId}');
    editorStyleLog.finer(' - extent: ${documentSelection.extent.nodeId}');

    if (documentSelection.base.nodeId == documentSelection.extent.nodeId) {
      editorStyleLog.finer(' - selection is within 1 node.');
      if (documentSelection.base.nodeId != node.id) {
        // Only 1 node is selected and its not the node we're interested in. Return.
        editorStyleLog.finer(' - this node is not selected. Returning null.');
        return null;
      }

      editorStyleLog.finer(' - this node has the selection');
      final baseNodePosition = documentSelection.base.nodePosition;
      final extentNodePosition = documentSelection.extent.nodePosition;
      late NodeSelection? nodeSelection;
      try {
        nodeSelection = node.computeSelection(
          base: baseNodePosition,
          extent: extentNodePosition,
        );
      } catch (exception) {
        // This situation can happen in the moment between a document change and
        // a corresponding selection change. For example: deleting an image and
        // replacing it with an empty paragraph. Between the doc change and the
        // selection change, the old image selection is applied to the new paragraph.
        // This results in an exception.
        //
        // TODO: introduce a unified event ledger that combines related behaviors
        //       into atomic transactions (#423)
        return null;
      }
      editorStyleLog.finer(' - node selection: $nodeSelection');

      return DocumentNodeSelection(
        nodeId: node.id,
        nodeSelection: nodeSelection,
        isBase: true,
        isExtent: true,
      );
    } else {
      // Log all the selected nodes.
      editorStyleLog.finer(' - selection contains multiple nodes:');
      for (final node in selectedNodes) {
        editorStyleLog.finer('   - ${node.id}');
      }

      if (selectedNodes.firstWhereOrNull(
            (selectedNode) => selectedNode.id == node.id,
          ) ==
          null) {
        // The document selection does not contain the node we're interested in. Return.
        editorStyleLog.finer(' - this node is not in the selection');
        return null;
      }

      if (selectedNodes.first.id == node.id) {
        editorStyleLog.finer(' - this is the first node in the selection');
        // Multiple nodes are selected and the node that we're interested in
        // is the top node in that selection. Therefore, this node is
        // selected from a position down to its bottom.
        final isBase = node.id == documentSelection.base.nodeId;
        return DocumentNodeSelection(
          nodeId: node.id,
          nodeSelection: node.computeSelection(
            base:
                isBase ? documentSelection.base.nodePosition : node.endPosition,
            extent:
                isBase
                    ? node.endPosition
                    : documentSelection.extent.nodePosition,
          ),
          isBase: isBase,
          isExtent: !isBase,
          highlightWhenEmpty: isBase,
        );
      } else if (selectedNodes.last.id == node.id) {
        editorStyleLog.finer(' - this is the last node in the selection');
        // Multiple nodes are selected and the node that we're interested in
        // is the bottom node in that selection. Therefore, this node is
        // selected from the beginning down to some position.
        final isBase = node.id == documentSelection.base.nodeId;
        return DocumentNodeSelection(
          nodeId: node.id,
          nodeSelection: node.computeSelection(
            base: isBase ? node.beginningPosition : node.beginningPosition,
            extent:
                isBase
                    ? documentSelection.base.nodePosition
                    : documentSelection.extent.nodePosition,
          ),
          isBase: isBase,
          isExtent: !isBase,
          highlightWhenEmpty: isBase,
        );
      } else {
        editorStyleLog.finer(
          ' - this node is fully selected within the selection',
        );
        // Multiple nodes are selected and this node is neither the top
        // or the bottom node, therefore this entire node is selected.
        return DocumentNodeSelection(
          nodeId: node.id,
          nodeSelection: node.computeSelection(
            base: node.beginningPosition,
            extent: node.endPosition,
          ),
          highlightWhenEmpty: true,
        );
      }
    }
  }
}
