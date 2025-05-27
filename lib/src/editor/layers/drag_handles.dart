import 'package:flutter/material.dart';
import 'package:flutter/src/widgets/framework.dart';
import 'package:follow_the_leader/follow_the_leader.dart';
import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/infrastructure/_logging.dart';

class NodeLeaderLayerBuilder extends SuperEditorLayerBuilder {
  NodeLeaderLayerBuilder({required this.nodeHandleLinks});

  final Map<String, LeaderLink> nodeHandleLinks;

  @override
  ContentLayerWidget build(
    BuildContext context,
    SuperEditorContext editContext,
  ) {
    return NodeLeaderUnderlay(
      nodeHandleLinks: nodeHandleLinks,
      document: editContext.document,
      documentLayout: editContext.documentLayout,
    );
  }
}

class NodeLeaderUnderlay extends DocumentLayoutLayerStatefulWidget {
  const NodeLeaderUnderlay({
    super.key,
    required this.nodeHandleLinks,
    required this.document,
    required this.documentLayout,
  });

  final Map<String, LeaderLink> nodeHandleLinks;
  final Document document;
  final DocumentLayout documentLayout;

  @override
  DocumentLayoutLayerState<ContentLayerStatefulWidget, void> createState() =>
      NodeLeaderUnderlayState();
}

class NodeLeaderUnderlayState
    extends DocumentLayoutLayerState<NodeLeaderUnderlay, void> {
  @override
  void computeLayoutDataWithDocumentLayout(
    BuildContext contentLayersContext,
    BuildContext documentContext,
    DocumentLayout documentLayout,
  ) {}

  @override
  Widget doBuild(BuildContext context, void layoutData) {
    return Stack(
      children: [
        for (final node in widget.document)
          Builder(
            builder: (context) {
              pluginNodeMetadataLog.finest('attaching link for ${node.id}!');
              final component = widget.documentLayout.getComponentByNodeId(
                node.id,
              );
              if (component == null) return const SizedBox();

              final componentBox =
                  component.context.findRenderObject() as RenderBox?;
              if (componentBox == null) return const SizedBox();

              widget.nodeHandleLinks[node.id] ??= LeaderLink();
              final layerBox = context.findRenderObject();
              if (layerBox == null) return const SizedBox();
              pluginNodeMetadataLog.finest('  about to position the LeaderLink!');
              final offset = componentBox.localToGlobal(Offset.zero);
              pluginNodeMetadataLog.finest('  $offset');
              final position = widget.documentLayout.getDocumentOffsetFromAncestorOffset(offset);
              pluginNodeMetadataLog.finest('  $position');

              return Positioned(
                left: offset.dx,
                top: position.dy,
                child: Leader(
                  link: widget.nodeHandleLinks[node.id]!,
                  child: const SizedBox.shrink()// (width: 10, height: 10,),
                ),
              );
            },
          ),
      ],
    );
  }
}

class NodeDragHandleLayerBuilder extends SuperEditorLayerBuilder {
  NodeDragHandleLayerBuilder({required this.nodeHandleLinks});

  final Map<String, LeaderLink> nodeHandleLinks;

  @override
  ContentLayerWidget build(
    BuildContext context,
    SuperEditorContext editContext,
  ) {
    return DragHandleOverlay(
      nodeHandleLinks: nodeHandleLinks,
      document: editContext.document,
      editor: editContext.editor,
    );
  }
}

class DragHandleOverlay extends ContentLayerStatefulWidget {
  DragHandleOverlay({
    super.key,
    required this.nodeHandleLinks,
    required this.document,
    required this.editor,
  });

  final Map<String, LeaderLink> nodeHandleLinks;
  final Document document;
  final Editor editor;

  @override
  ContentLayerState<DragHandleOverlay, void> createState() =>
      DragHandleOverlayState();
}

class DragHandleOverlayState
    extends ContentLayerState<DragHandleOverlay, void> {
  @override
  void computeLayoutData(Element? contentElement, RenderObject? contentLayout) {
    // TODO: implement computeLayoutData
  }

  @override
  Widget doBuild(BuildContext context, void layoutData) {
    return Stack(
      children: [
        for (final entry in widget.nodeHandleLinks.entries)
          Follower.withAligner(
            link: entry.value,
            aligner: StaticOffsetAligner(
              leaderAnchor: Alignment.centerLeft,
              followerAnchor: Alignment.topRight,
              // offset: Offset(0, -20),
              offset: Offset.zero,
            ),
            // showDebugPaint: true,
            // showWhenUnlinked: true,
            // repaintWhenLeaderChanges: true,
            child: DragHandle(nodeID: entry.key, editor: widget.editor),
          ),
      ],
    );
  }
}

class DragHandle extends StatefulWidget {
  const DragHandle({super.key, required this.nodeID, required this.editor});

  final String nodeID;
  final Editor editor;

  @override
  State<DragHandle> createState() => DragHandleState();
}

class DragHandleState extends State<DragHandle> {
  bool _isDragging = false;

  @override
  Widget build(BuildContext context) {
    pluginNodeMetadataLog.finest('DragHandle for ${widget.nodeID}');
    pluginNodeMetadataLog.finest('  default coloring: ${Theme.of(context).primaryColor}');
    return GestureDetector(
      behavior: HitTestBehavior.translucent,
      onPanStart: (_) => setState(() => _isDragging = true),
      onPanUpdate: (details) {
        // TODO handle node reordering/whatnot
      },
      onPanEnd: (_) => setState(() => _isDragging = false),
      child: MouseRegion(
        cursor: SystemMouseCursors.grab,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 150),
          // TODO figure out how to get the handle to vertically align with
          //      the first line
          padding: EdgeInsets.symmetric(vertical: 4),
          // decoration: BoxDecoration(
          // ),
          child: Icon(
            Icons.drag_indicator,
            size: 18,
            applyTextScaling: true,
            semanticLabel: 'Drag Handle',
            color:
                _isDragging
                    ? Theme.of(context).iconTheme.color
                    : Theme.of(context).primaryColor,
          ),
        ),
      ),
    );
  }
}
