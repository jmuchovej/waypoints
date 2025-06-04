import 'package:flutter/material.dart';
import 'package:tempo/src/editor/super_editor.dart';

import '_manager_hover.dart';

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
