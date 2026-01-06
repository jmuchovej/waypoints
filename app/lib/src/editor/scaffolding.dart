import 'package:follow_the_leader/follow_the_leader.dart';

import 'scaffolding/config.dart';
import 'scaffolding/elements/drag_handle.dart';
import 'scaffolding/elements/switch_node_type.dart';
import 'scaffolding/layers/hover.dart';
import 'scaffolding/layers/scaffolding.dart';
import 'scaffolding/managers/hover.dart';
import 'scaffolding/managers/link.dart';
import 'super_editor.dart';

export 'scaffolding/components.dart' show ScaffoldedComponentBuilder;
export 'scaffolding/config.dart' show ScaffoldConfig, DefaultScaffoldConfig;
export 'scaffolding/managers/hover.dart' show HoverManager;
export 'scaffolding/managers/link.dart' show LeaderLinkManager;

/// The [ScaffoldingPlugin] provides a way to anchor [Follower]s to
/// [LeaderLink]s. This allows us to attach widgets like [DragHandle],
/// [SwitchNodeType], and many others.
///
/// This is achieved by coordinating a [HoverManager] with registered
/// [linkManager] and ensuring that we attach a [HoverTrackingLayer] so that we
/// can ensure the mouse position is accurately traced.
class ScaffoldingPlugin extends SuperEditorPlugin {
  ScaffoldingPlugin({
    required this.hoverManager,
    required this.linkManager,
    required this.showDebugPaint,
    this.scaffoldConfig,
  });

  final HoverManager hoverManager;
  final LeaderLinkManager linkManager;
  final bool showDebugPaint;
  final ScaffoldConfig? scaffoldConfig;

  @override
  List<SuperEditorLayerBuilder> get documentOverlayBuilders => [
    HoverTrackingLayerBuilder(hoverManager: hoverManager),
    ScaffoldingLayerBuilder(
      linkManager: linkManager,
      hoverManager: hoverManager,
      scaffoldConfig: scaffoldConfig,
    ),
  ];
  @override
  List<SuperEditorLayerBuilder> get documentUnderlayBuilders => const [];
}
