import '../things/thing/thing.entity.dart';

/// Base class for all panel requests
abstract class PanelRequest {
  const PanelRequest();
}

/// Request to open a Thing entity in a panel
class OpenThingRequest extends PanelRequest {
  const OpenThingRequest({
    required this.uuid,
    required this.type,
    this.metadata,
  });

  factory OpenThingRequest.fromThing(ThingEntity thing) {
    return OpenThingRequest(uuid: thing.uuid, type: thing.type);
  }

  final String uuid;
  final String type;
  final Map<String, dynamic>? metadata;
}

/// Request to close a panel
class ClosePanelRequest extends PanelRequest {
  const ClosePanelRequest({this.panelID});

  final String? panelID;
}

/// Request to switch to a different panel
class SwitchPanelRequest extends PanelRequest {
  const SwitchPanelRequest({required this.panelID});

  final String panelID;
}

/// Request to toggle sidebar visibility
class ToggleSidebarRequest extends PanelRequest {
  const ToggleSidebarRequest();
}

/// Request to open detail pane
class OpenDetailPaneRequest extends PanelRequest {
  const OpenDetailPaneRequest({required this.route, this.args});

  final String route;
  final Map<String, dynamic>? args;
}

/// Request to close detail pane
class CloseDetailPaneRequest extends PanelRequest {
  const CloseDetailPaneRequest();
}

/// Configuration for how a panel should be opened
class PanelConfig {
  const PanelConfig({
    this.allowMultiple = false,
    this.replaceExisting = true,
    this.position = PanelPosition.E,
    this.width,
  });

  final bool allowMultiple;
  final bool replaceExisting;
  final PanelPosition position;
  final double? width;
}

enum PanelPosition { N, E, S, W, floating }
