import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../super_editor.dart';
import '../../things/thing/interface.dart';
import 'thing_node_position.dart';
import '../../infrastructure/logging.dart';

/// Defines different cursor zones within a ThingNode component
enum ThingCursorZone {
  /// Text editing areas (title, description, etc.)
  textEditable(SystemMouseCursors.text),

  /// Interactive elements (buttons, chips, pills)
  interactive(SystemMouseCursors.click),

  /// Clickable but not interactive elements (navigation areas)
  clickable(SystemMouseCursors.click),

  /// Default/fallback areas
  basic(SystemMouseCursors.basic);

  const ThingCursorZone(this.cursor);
  final MouseCursor cursor;
}

/// Interface for ThingNode components to define their cursor zones
abstract mixin class ThingCursorProvider {
  /// Returns the appropriate cursor zone for a given local offset within the component
  ThingCursorZone getCursorZoneForOffset(Offset localOffset);
}

/// Content tap handler that provides custom mouse cursors for all ThingNodes
///
/// This handler determines the appropriate cursor based on where the mouse
/// is hovering within any ThingNode using simple position-based heuristics.
class ThingCursorHandler extends ContentTapDelegate {
  ThingCursorHandler({
    required this.editContext,
    required this.context,
    required this.ref,
    this.panelInfo,
  });

  final SuperEditorContext editContext;
  final BuildContext context;
  final WidgetRef ref;
  final dynamic panelInfo; // Panel context (using dynamic to avoid import issues)
  DocumentLayout get documentLayout => editContext.documentLayout;

  @override
  MouseCursor? mouseCursorForContentHover(DocumentPosition hoverPosition) {
    // Get the node at this position
    final nodeId = hoverPosition.nodeId;
    final document = editContext.document;
    final node = document.getNodeById(nodeId);

    tempoLog.finest(
      '${node.runtimeType} is a `ThingNode`? ${node is ThingNode}!',
    );
    if (node is! ThingNode) {
      // Not a thing node, let other handlers decide
      return null;
    }

    // Check if the position has cursor zone metadata
    final nodePosition = hoverPosition.nodePosition;

    if (nodePosition is ThingNodePosition) {
      // We have explicit zone information, use it
      tempoLog.finest('ThingNodePosition with zone: ${nodePosition.zone}');
      return nodePosition.zone.cursor;
    }

    // Fallback: regular TextNodePosition means it's in text area
    if (nodePosition is TextNodePosition) {
      tempoLog.finest('Regular TextNodePosition, returning text cursor');
      return SystemMouseCursors.text;
    }

    // Shouldn't reach here, but default to click cursor
    tempoLog.warning(
      'Unexpected node position type: ${nodePosition.runtimeType}',
    );
    return SystemMouseCursors.click;
  }

  @override
  TapHandlingInstruction onTap(DocumentTapDetails details) {
    // Get the document position from the tap
    final tapPosition = details.documentLayout
        .getDocumentPositionNearestToOffset(details.layoutOffset);
    if (tapPosition == null) {
      return TapHandlingInstruction.continueHandling;
    }

    // Get the node at this position
    final nodeId = tapPosition.nodeId;
    final document = editContext.document;
    final node = document.getNodeById(nodeId);

    if (node is! ThingNode) {
      // Not a thing node, let other handlers decide
      return TapHandlingInstruction.continueHandling;
    }

    // Check if this is a ThingNodePosition with non-text zone
    final nodePosition = tapPosition.nodePosition;
    if (nodePosition is ThingNodePosition && !nodePosition.isTextEditable) {
      // This is a non-text area, handle navigation
      tempoLog.info(
        '🚀 Tap in ${nodePosition.zone} zone, navigating to thing: ${node.thing.uuid}',
      );

      // Traditional navigation - push new page onto stack
      _navigateToThing(node.thing.uuid);

      // Halt further processing since we handled the navigation
      return TapHandlingInstruction.halt;
    }

    // This is a text area or regular TextNodePosition, let SuperEditor handle it
    tempoLog.info('🎯 Tap in text area, letting SuperEditor handle');
    return TapHandlingInstruction.continueHandling;
  }

  /// Navigate to thing using traditional push navigation
  void _navigateToThing(String uuid) {
    // Traditional navigation - always navigate to standalone entity page
    // This creates a proper navigation stack where back button works naturally
    context.push('/thing/$uuid');
    tempoLog.info('Traditional navigation push to: /thing/$uuid');
  }
}

/// Extension to easily check if a component supports cursor zones
extension ThingComponentExtensions on State {
  bool get supportsCursorZones => this is ThingCursorProvider;

  ThingCursorProvider? get asCursorProvider =>
      this is ThingCursorProvider ? this as ThingCursorProvider : null;
}
