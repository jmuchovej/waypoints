import 'package:flutter/material.dart';
import '../../super_editor.dart';

class HoverManager extends ChangeNotifier implements Editable {
  static final String key = 'HoverManager';

  String? _hoveredNodeID;
  String? get hoveredNodeID => _hoveredNodeID;
  bool isHovered(String nodeID) => hoveredNodeID == nodeID;

  void updateEditorBounds(RenderObject? contentLayout) {
    if (contentLayout == null) return;
    if (contentLayout is! RenderBox) {
      // TODO implement error-handling if this happens?
      return;
    }
  }

  void updateMousePosition(Offset localPosition, SuperEditorContext context) {
    String? newHoveredNodeID;

    // devtoolsLog.finest('docPadding: ${context..documentPadding}');

    for (final node in context.document) {
      final component = context.documentLayout.getComponentByNodeId(node.id);
      if (component == null) continue;

      final box = component.context.findRenderObject() as RenderBox?;
      if (box == null || !box.hasSize) continue;

      final offset = box.localToGlobal(Offset.zero);
      final bounds = offset & box.size;

      if (bounds.top <= localPosition.dy && localPosition.dy <= bounds.bottom) {
        newHoveredNodeID = node.id;
        break;
      }
    }

    if (newHoveredNodeID != hoveredNodeID) {
      _hoveredNodeID = newHoveredNodeID;
      notifyListeners();
    }
  }

  void clearHover() {
    if (hoveredNodeID != null) {
      _hoveredNodeID = null;
      notifyListeners();
    }
  }

  @override
  void onTransactionEnd(List<EditEvent> edits) {}

  @override
  void onTransactionStart() {}

  @override
  void reset() {
    clearHover();
  }
}
