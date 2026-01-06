import 'package:flutter/material.dart';
import 'package:follow_the_leader/follow_the_leader.dart';

import '../../super_editor.dart';
import '../config.dart';
import '../managers/hover.dart';
import '../managers/link.dart';

/// The [ScaffoldingLayerBuilder] simply assembles the [ScaffoldingOverlay],
/// see those docs for more details!
class ScaffoldingLayerBuilder extends SuperEditorLayerBuilder {
  ScaffoldingLayerBuilder({
    required this.hoverManager,
    required this.linkManager,
    this.scaffoldConfig,
  });

  final HoverManager hoverManager;
  final LeaderLinkManager linkManager;
  final ScaffoldConfig? scaffoldConfig;

  @override
  ContentLayerWidget build(
    BuildContext context,
    SuperEditorContext editContext,
  ) {
    return ScaffoldingOverlay(
      document: editContext.document,
      composer: editContext.composer,
      linkManager: linkManager,
      hoverManager: hoverManager,
      scaffoldConfig: scaffoldConfig,
    );
  }
}

/// The [ScaffoldingOverlay] tracks the actively hovered widget (via the
/// [HoverManager]) and dispatches painting the various [Follower] widgets.
///
/// Notably, this [ScaffoldingOverlayState] does the following:
/// - render scaffold elements for only _active_ (hovered/selected) nodes
/// - use [Follower] widgets for position relative to [Leader]s in components
/// - responds to both hover state and [DocumentSelection] changes
///
/// TODO allow scaffolding elements to be configurable per node-type
/// TODO add an animation system for smooth show/hide transitions
class ScaffoldingOverlay extends ContentLayerStatefulWidget {
  const ScaffoldingOverlay({
    super.key,
    required this.document,
    required this.composer,
    required this.linkManager,
    required this.hoverManager,
    this.scaffoldConfig,
  });

  final Document document;
  final DocumentComposer composer;
  final HoverManager hoverManager;
  final LeaderLinkManager linkManager;
  final ScaffoldConfig? scaffoldConfig;

  @override
  ContentLayerState<ContentLayerStatefulWidget, void> createState() =>
      ScaffoldingOverlayState();
}

class ScaffoldingOverlayState
    extends ContentLayerState<ScaffoldingOverlay, void> {
  @override
  void computeLayoutData(
    Element? contentElement,
    RenderObject? contentLayout,
  ) {}

  @override
  void dispose() {
    widget.hoverManager.removeListener(_rebuild);
    widget.composer.selectionNotifier.removeListener(_rebuild);
    super.dispose();
  }

  @override
  Widget doBuild(BuildContext context, void layoutData) {
    return Stack(
      children: [
        for (final node in widget.document)
          if (_isActive(node.id)) ..._buildScaffolding(context, node),
      ],
    );
  }

  @override
  void initState() {
    super.initState();
    widget.hoverManager.addListener(_rebuild);
    widget.composer.selectionNotifier.addListener(_rebuild);
  }

  List<Widget> _buildScaffolding(BuildContext context, DocumentNode node) {
    final link = widget.linkManager.getLink(node.id);
    final config = widget.scaffoldConfig ?? DefaultScaffoldConfig();
    final elements = config.getElements(node);

    return elements.map((e) => e.build(context, node.id, link)).toList();
  }

  bool _isActive(String nodeID) {
    if (widget.hoverManager.isHovered(nodeID)) return true;

    final s = widget.composer.selection; // sugar
    return s != null && (s.base.nodeId == nodeID || s.extent.nodeId == nodeID);
  }

  void _rebuild() => setState(() {});
}
