import 'package:flutter/material.dart';
import 'package:follow_the_leader/follow_the_leader.dart';
import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/editor/widgets/drag_handle.dart';

import './_manager_link.dart';

class ScaffoldedComponentWrapper extends StatefulWidget {
  const ScaffoldedComponentWrapper({
    super.key,
    required this.nodeId,
    required this.linkManager,
    required this.child,
  });

  final String nodeId;
  final Widget child;
  final LeaderLinkManager linkManager;

  @override
  State<ScaffoldedComponentWrapper> createState() =>
      _ScaffoldedComponentWrapperState();
}

class _ScaffoldedComponentWrapperState
    extends State<ScaffoldedComponentWrapper> {
  late final LeaderLink _link;

  @override
  void initState() {
    super.initState();
    _link = widget.linkManager.getLink(widget.nodeId);
    widget.linkManager.retainLink(widget.nodeId);
  }

  @override
  void dispose() {
    widget.linkManager.releaseLink(widget.nodeId);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Leader(link: _link, child: widget.child);
  }
}

class ScaffoldedComponentBuilder extends ComponentBuilder {
  ScaffoldedComponentBuilder({
    required this.innerBuilder,
    required this.linkManager,
  });

  final ComponentBuilder innerBuilder;
  final LeaderLinkManager linkManager;

  @override
  SingleColumnLayoutComponentViewModel? createViewModel(
    Document document,
    DocumentNode node,
  ) {
    return innerBuilder.createViewModel(document, node);
  }

  @override
  Widget? createComponent(
    SingleColumnDocumentComponentContext componentContext,
    SingleColumnLayoutComponentViewModel componentViewModel,
  ) {
    final child = innerBuilder.createComponent(
      componentContext,
      componentViewModel,
    );
    if (child == null) return null;

    final link = linkManager.getLink(componentViewModel.nodeId);

    return Leader(link: link, child: child);
  }
}
