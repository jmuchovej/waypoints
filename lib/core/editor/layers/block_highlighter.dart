import 'package:flutter/material.dart';
import 'package:tempo/core/editor/super_editor.dart';
import 'package:tempo/infrastructure/_logging.dart';

class BlockHighlightBuilder implements SuperEditorLayerBuilder {
  const BlockHighlightBuilder();

  @override
  ContentLayerWidget build(
    BuildContext context,
    SuperEditorContext editContext,
  ) {
    return BlockHighlightLayer(
      color: Theme.of(context).highlightColor,
      duration: const Duration(milliseconds: 150),
      documentLayout: () => editContext.documentLayout,
    );
  }
}

class BlockHighlightLayer extends DocumentLayoutLayerStatefulWidget {
  const BlockHighlightLayer({
    super.key,
    required this.color,
    required this.duration,
    required this.documentLayout,
  });

  final Color color;
  final Duration duration;
  final DocumentLayoutResolver documentLayout;

  @override
  DocumentLayoutLayerState<ContentLayerStatefulWidget, Rect?> createState() =>
      BlockHighlightLayerState();
}

class BlockHighlightLayerState
    extends DocumentLayoutLayerState<BlockHighlightLayer, Rect?>
    with SingleTickerProviderStateMixin {
  String? _targetNodeID;
  Rect? _targetRect;
  late AnimationController _animationController;
  late Animation<double> _fadeAnimation;

  @override
  void initState() {
    super.initState();
    _animationController = AnimationController(
      vsync: this,
      duration: Duration(milliseconds: 150),
    );
    _fadeAnimation = CurvedAnimation(
      parent: _animationController,
      curve: Curves.easeOut,
    );
  }

  @override
  void dispose() {
    _animationController.dispose();
    super.dispose();
  }

  @override
  Rect? computeLayoutDataWithDocumentLayout(
    BuildContext contentLayersContext,
    BuildContext documentContext,
    DocumentLayout documentLayout,
  ) {
    if (_targetNodeID == null) return null;

    final component = documentLayout.getComponentByNodeId(_targetNodeID!);
    if (component == null) return null;

    final componentBox = component.context.findRenderObject() as RenderBox?;
    if (componentBox == null) return null;

    final documentBox = documentContext.findRenderObject() as RenderObject;
    final componentOffset = componentBox.localToGlobal(
      Offset.zero,
      ancestor: documentBox,
    );

    final window = contentLayersContext.findRenderObject() as RenderBox;

    final height = componentBox.size.height;
    final Rect rect = Rect.fromLTRB(
      0,
      componentOffset.dy - height * 0.1,
      window.size.width,
      componentOffset.dy + componentBox.size.height * 1.1,
    );
    return rect;
  }

  void _onHoverUpdate(Offset globalOffset, DocumentLayout layout) {
    layerBlockHighlighterLog.finest('_onHoverUpdate:');
    // Convert `globalPosition` to a document offset
    final documentOffset = layout.getDocumentOffsetFromAncestorOffset(
      globalOffset,
    );
    layerBlockHighlighterLog.finest('  documentOffset: ${documentOffset}');

    // find the nearest document position (block) at that offset
    final nearestDocumentPosition = layout.getDocumentPositionNearestToOffset(
      documentOffset,
    );
    layerBlockHighlighterLog.finest(
      '  nearestPosition: ${nearestDocumentPosition}',
    );

    final actualDocumentPosition = layout.getDocumentPositionAtOffset(
      documentOffset,
    );
    layerBlockHighlighterLog.finest(
      '  actualPosition : ${actualDocumentPosition}',
    );

    // We're not even near a document node!
    if (nearestDocumentPosition == null) return;

    String? nodeID = nearestDocumentPosition.nodeId;
    // When `nearest` and `actual` document positions conflict, check if we're
    // at the same `dy`!
    if (nearestDocumentPosition.nodeId != actualDocumentPosition?.nodeId) {
      final component = layout.getComponentByNodeId(
        nearestDocumentPosition.nodeId,
      );
      // There's no component, or it's `RenderObject` hasn't attached yet!
      if (component == null || component.context.findRenderObject() == null)
        return;
      final componentBox = component.context.findRenderObject() as RenderBox;
      final componentOffset = layout.getDocumentOffsetFromAncestorOffset(
        componentBox.localToGlobal(Offset.zero),
      );
      final componentRect = componentOffset & componentBox.size;

      if (!componentRect.contains(
        Offset(componentRect.left, documentOffset.dy),
      )) {
        nodeID = null;
      }
    }

    // They're the same, so we don't need to change anything!
    if (_targetNodeID == nodeID) return;

    setState(() {
      _targetNodeID = nodeID;
      if (_targetNodeID == null) {
        _animationController.reverse();
      } else {
        _animationController.forward();
      }
    });
  }

  @override
  Widget doBuild(BuildContext context, Rect? layoutData) {
    _targetRect = layoutData;

    return MouseRegion(
      opaque: false,
      onHover:
          (event) => _onHoverUpdate(event.position, widget.documentLayout()),
      onExit:
          (_) => setState(() {
            _targetNodeID = null;
            _animationController.reverse();
          }),
      child: AnimatedBuilder(
        animation: _fadeAnimation,
        builder: (context, child) {
          if (_fadeAnimation.value == 0) return const SizedBox.expand();
          if (_targetRect == null) return const SizedBox.expand();

          return CustomPaint(
            size: Size.infinite,
            painter: BlockHighlightPainter(
              rect: _targetRect!,
              color: ColorScheme.of(
                context,
              ).secondaryContainer.withValues(alpha: _fadeAnimation.value),
            ),
          );
        },
      ),
    );
  }
}

class BlockHighlightPainter extends CustomPainter {
  BlockHighlightPainter({required this.rect, required this.color});

  final Rect rect;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint =
        Paint()
          ..color = color
          ..style = PaintingStyle.fill;

    final roundedRect = RRect.fromRectAndRadius(
      rect,
      const Radius.circular(4.0),
    );

    canvas.drawRRect(roundedRect, paint);
  }

  @override
  bool shouldRepaint(BlockHighlightPainter oldPainter) {
    return rect != oldPainter.rect || color != oldPainter.color;
  }
}
