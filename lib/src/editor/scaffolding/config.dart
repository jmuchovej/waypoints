import 'package:flutter/widgets.dart';
import 'package:follow_the_leader/follow_the_leader.dart';

import '../nodes.dart';
import '../super_editor.dart';
import 'elements/drag_handle.dart';
import 'elements/switch_node_type.dart';

class DefaultScaffoldConfig extends ScaffoldConfig {
  @override
  List<ScaffoldElement> getElements(DocumentNode node) {
    // Don't show scaffolding elements for DocumentHeaderNode
    if (node is DocumentHeaderNode) {
      return [];
    }

    // For all other nodes, show the default scaffolding elements
    final elements = <ScaffoldElement>[];

    elements.add(DragHandleElement());
    elements.add(SwitchNodeTypeElement());

    return elements;
  }
}

abstract class ScaffoldConfig {
  List<ScaffoldElement> getElements(DocumentNode node);
}

abstract class ScaffoldElement {
  Widget build(BuildContext context, String nodeID, LeaderLink link);
}
