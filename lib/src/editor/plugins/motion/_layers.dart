import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/editor/plugins/motion/motion.dart';
import 'package:flutter/material.dart';

class MotionIndicatorOverlayBuilder extends SuperEditorLayerBuilder {
  MotionIndicatorOverlayBuilder({required this.mode});

  final Motion mode;

  @override
  ContentLayerWidget build(
    BuildContext context,
    SuperEditorContext editContext,
  ) {
    return ContentLayerProxyWidget(
      child: IgnorePointer(
        child: RepaintBoundary(
          child: Stack(
            clipBehavior: Clip.none,
            children: [
              Positioned(
                top: 8,
                left: 8,
                child: ValueListenableBuilder(
                  valueListenable: mode,
                  builder:
                      (context, mode, child) => Row(
                        children: [
                          Container(
                            padding: EdgeInsets.symmetric(
                              horizontal: 8,
                              vertical: 4,
                            ),
                            decoration: BoxDecoration(
                              color: mode.color,
                              borderRadius: BorderRadius.circular(8),
                            ),
                            child: Text(
                              mode.toDisplayString(),
                              style: const TextStyle(
                                color: Colors.white,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                          ),
                        ],
                      ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class NodeSelectionUnderlayBuilder extends SuperEditorLayerBuilder {
  NodeSelectionUnderlayBuilder({
    required this.normalModeColor,
    required this.insertModeColor,
  });

  final Color normalModeColor;
  final Color insertModeColor;

  String? currentNodeID;

  @override
  ContentLayerWidget build(
    BuildContext context,
    SuperEditorContext editContext,
  ) {
    return NodeSelectionUnderlay(
      normalModeColor: normalModeColor,
      insertModeColor: insertModeColor,
      documentLayoutResolver: () => editContext.documentLayout,
      composer: editContext.composer,
      document: editContext.document,
    );
  }
}

/// NAVIGATION CONTEXT PROVIDER: Captures BuildContext for Thing navigation
///
/// This layer doesn't render anything visible, but provides a BuildContext
/// that the motion plugin can use for navigation. This solves the problem
/// of ContentTapDelegate not having access to a BuildContext.
class MotionNavigationContextProvider extends SuperEditorLayerBuilder {
  MotionNavigationContextProvider({required this.contentTapDelegate});

  final MotionContentTapDelegate contentTapDelegate;

  @override
  ContentLayerWidget build(
    BuildContext context,
    SuperEditorContext editContext,
  ) {
    // Store the context in the tap delegate for navigation
    // contentTapDelegate.context = context;

    return ContentLayerProxyWidget(
      child: const SizedBox.shrink(), // Invisible widget
    );
  }
}

class NodeSelectionUnderlay extends DocumentLayoutLayerStatefulWidget {
  const NodeSelectionUnderlay({
    super.key,
    required this.composer,
    required this.documentLayoutResolver,
    required this.document,
    required this.normalModeColor,
    required this.insertModeColor,
  });

  final Color normalModeColor;
  final Color insertModeColor;
  final DocumentLayout Function() documentLayoutResolver;
  final DocumentComposer composer;
  final Document document;

  @override
  DocumentLayoutLayerState<NodeSelectionUnderlay, Rect?> createState() =>
      NodeSelectionUnderlayState();
}

class NodeSelectionUnderlayState
    extends DocumentLayoutLayerState<NodeSelectionUnderlay, Rect?> {
  String? _targetNodeID;
  Rect? _targetRect;

  @override
  Rect? computeLayoutDataWithDocumentLayout(
    BuildContext contentLayersContext,
    BuildContext documentContext,
    DocumentLayout documentLayout,
  ) {
    final selection = widget.composer.selection;
    if (selection == null) {
      return null;
    }

    final nodeID = selection.extent.nodeId;
    final node = widget.document.getNodeById(nodeID);
    final component = documentLayout.getComponentByNodeId(nodeID);
    if (node == null || component == null) {
      return null;
    }

    Rect? bounds = documentLayout.getRectForSelection(
      DocumentPosition(nodeId: nodeID, nodePosition: node.beginningPosition),
      DocumentPosition(nodeId: nodeID, nodePosition: node.endPosition),
    );

    Rect? downstream = documentLayout.getEdgeForPosition(
      DocumentPosition(
        nodeId: nodeID,
        nodePosition: TextNodePosition(offset: 0),
      ),
    );

    Rect? upstream = documentLayout.getEdgeForPosition(
      DocumentPosition(
        nodeId: nodeID,
        nodePosition: selection.extent.nodePosition,
      ),
    );

    if (bounds == null || downstream == null || upstream == null) {
      return null;
    }

    Rect rect = Rect.fromLTWH(
      bounds.left,
      bounds.top,
      bounds.width,
      // upstream.right - downstream.right,
      bounds.height,
      // upstream.bottom - downstream.top,
    );

    return rect;
  }

  @override
  Widget doBuild(BuildContext context, Rect? motionHighlight) {
    return IgnorePointer(
      child: Stack(
        clipBehavior: Clip.none,
        children: [
          if (motionHighlight != null)
            Positioned(
              top: motionHighlight.top,
              left: motionHighlight.left,
              height: motionHighlight.height,
              child: Container(
                width: motionHighlight.width,
                decoration: BoxDecoration(
                  color:
                      motionMode.value == MotionMode.normal
                          ? widget.normalModeColor
                          : widget.insertModeColor.withOpacity(0.3),
                  borderRadius: BorderRadius.circular(8.0),
                ),
              ),
            ),
        ],
      ),
    );
  }
}
