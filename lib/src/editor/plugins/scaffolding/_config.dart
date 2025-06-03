import 'package:flutter/widgets.dart';
import 'package:follow_the_leader/follow_the_leader.dart';
import 'package:tempo/src/editor/super_editor.dart';
import '_elements/_drag_handle.dart';
import '_elements/_switch_node_type.dart';

abstract class ScaffoldConfig {
  List<ScaffoldElement> getElements(DocumentNode node);
}

abstract class ScaffoldElement {
  Widget build(BuildContext context, String nodeID, LeaderLink link);
}

class DefaultScaffoldConfig extends ScaffoldConfig {
  @override
  List<ScaffoldElement> getElements(DocumentNode node) {
    final elements = <ScaffoldElement>[];

    elements.add(DragHandleElement());
    elements.add(SwitchNodeTypeElement());

    return elements;
  }
}
