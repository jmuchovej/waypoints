import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:tempo/src/editor/interaction/_hover_manager.dart';
import 'package:tempo/src/editor/super_editor.dart';

class HoverTrackingLayerBuilder extends SuperEditorLayerBuilder {
  HoverTrackingLayerBuilder({required this.hoverManager});
  final HoverManager hoverManager;

  @override
  ContentLayerWidget build(
    BuildContext context,
    SuperEditorContext editContext,
  ) {
    return HoverTrackingLayer(
      hoverManager: hoverManager,
      editContext: editContext,
    );
  }
}

class HoverTrackingLayer extends ContentLayerStatelessWidget {
  const HoverTrackingLayer({
    super.key,
    required this.hoverManager,
    required this.editContext,
  });

  final HoverManager hoverManager;
  final SuperEditorContext editContext;

  @override
  Widget doBuild(
    BuildContext context,
    Element? contentElement,
    RenderObject? contentLayout,
  ) {
    hoverManager.updateEditorBounds(contentLayout);

    return MouseRegion(
      opaque: false,
      key: const Key('hover-tracker-mouse-region'),
      onHover:
          (event) =>
              hoverManager.updateMousePosition(event.position, editContext),
      onExit: (_) => hoverManager.clearHover(),
      child: IgnorePointer(
        child: Container(
          key: const Key('hover-tracker-sized-box'),
          // color: Colors.red.withValues(alpha: 0.2),
          child: const SizedBox.expand(), // invisible overlay
        ),
      ),
    );
  }
}

class DebugHoverLayerBuilder extends SuperEditorLayerBuilder {
  DebugHoverLayerBuilder({required this.hoverManager, required this.enabled});

  final HoverManager hoverManager;
  final bool enabled;

  @override
  ContentLayerWidget build(
    BuildContext context,
    SuperEditorContext editContext,
  ) {
    if (!enabled) return const EmptyContentLayer();

    return DebugHoverLayer(
      hoverManager: hoverManager,
      editContext: editContext,
    );
  }
}

class DebugHoverLayer extends ContentLayerStatelessWidget {
  const DebugHoverLayer({
    super.key,
    required this.hoverManager,
    required this.editContext,
  });

  final HoverManager hoverManager;
  final SuperEditorContext editContext;

  @override
  Widget doBuild(
    BuildContext context,
    Element? contentElement,
    RenderObject? contentLayout,
  ) {
    final editorBox = switch (contentLayout) {
      (RenderSliverToBoxAdapter _) => contentLayout.child,
      (RenderBox _) => contentLayout,
      (_) => null,
    };

    return IgnorePointer(
      child: CustomPaint(
        painter: HoverZonePainter(
          hoverManager: hoverManager,
          context: editContext,
          editorBox: editorBox,
        ),
        size: Size.infinite,
      ),
    );
  }
}

/// A custom painter to show hover zones for a given [DocumentNode]. This is
/// primarily useful for determining hover detection issues.
///
/// Visually:
/// - Full-width horizontal zone with ~10% opacity
/// - Component bounds drawn with opaque stroke
/// - Hover is denoted by a color change on full-width zone and component bounds
class HoverZonePainter extends CustomPainter {
  HoverZonePainter({
    required this.hoverManager,
    required this.context,
    required this.editorBox,
  }) : super(repaint: hoverManager);

  final HoverManager hoverManager;
  final SuperEditorContext context;
  final RenderBox? editorBox;

  @override
  void paint(Canvas canvas, Size size) {
    for (final node in context.document) {
      final component = context.documentLayout.getComponentByNodeId(node.id);
      if (component == null) continue;

      final box = component.context.findRenderObject() as RenderBox?;
      if (box == null || !box.hasSize) continue;

      final offset = box.localToGlobal(Offset.zero, ancestor: editorBox);
      final bounds = offset & box.size;

      final paint =
          Paint()
            ..color =
                hoverManager.isHovered(node.id) ? Colors.blue : Colors.grey;

      // Draw the edge-to-edge horizontal hover zone
      canvas.drawRect(
        Rect.fromLTRB(0, bounds.top, size.width, bounds.bottom),
        Paint.from(paint)
          ..color = paint.color.withValues(alpha: 0.1)
          ..style = PaintingStyle.fill,
      );

      // Draw the component's boundaries
      canvas.drawRect(
        bounds,
        Paint.from(paint)
          ..style = PaintingStyle.stroke
          ..strokeWidth = 1,
      );
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => true;
}
