import 'package:follow_the_leader/follow_the_leader.dart';
import 'package:tempo/src/editor/super_editor.dart';

import '_elements/_switch_node_type.dart';
import '_elements/_drag_handle.dart';
import '_manager_link.dart';
import '_manager_hover.dart';
import '_layer_scaffold.dart';
import '_layer_hover.dart';

export '_components.dart' show ScaffoldedComponentBuilder;
export '_manager_link.dart' show LeaderLinkManager;
export '_manager_hover.dart' show HoverManager;

/// The [ScaffoldingPlugin] provides a way to anchor [Follower]s to
/// [LeaderLink]s. This allows us to attach widgets like [DragHandle],
/// [SwitchNodeType], and many others.
///
/// This is achieved by coordinating a [HoverManager] with registered
/// [linkManager] and ensuring that we attach a [HoverTrackingLayer] so that we
/// can ensure the mouse position is accurately traced.
class ScaffoldingPlugin extends SuperEditorPlugin {
  const ScaffoldingPlugin({
    required this.hoverManager,
    required this.linkManager,
    required this.showDebugPaint,
  });

  final HoverManager hoverManager;
  final LeaderLinkManager linkManager;
  final bool showDebugPaint;

  @override
  List<SuperEditorLayerBuilder> get documentOverlayBuilders => [
    HoverTrackingLayerBuilder(hoverManager: hoverManager),
    ScaffoldingLayerBuilder(
      linkManager: linkManager,
      hoverManager: hoverManager,
    ),
  ];
  @override
  List<SuperEditorLayerBuilder> get documentUnderlayBuilders => const [];
}
