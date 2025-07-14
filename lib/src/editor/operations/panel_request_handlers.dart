import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../super_editor.dart';
import 'panel_requests.dart';
import 'panel_commands.dart';

/// Creates a panel request handler with access to context and ref for navigation
///
/// This function returns a handler closure that captures the BuildContext and WidgetRef,
/// allowing the handler to create commands that can navigate outside the SuperEditor tree.
EditRequestHandler createPanelRequestHandler(
  BuildContext context,
  WidgetRef ref,
) {
  return (Editor editor, EditRequest request) {
    if (request is OpenTaskPanelRequest) {
      return OpenTaskPanelCommand(
        taskUuid: request.taskUuid,
        context: context,
        ref: ref,
      );
    }

    if (request is OpenProjectPanelRequest) {
      return OpenProjectPanelCommand(
        projectUuid: request.projectUuid,
        context: context,
        ref: ref,
      );
    }

    if (request is NavigateToRouteRequest) {
      return NavigateToRouteCommand(
        routePath: request.routePath,
        context: context,
        parameters: request.parameters,
      );
    }

    // Return null for requests this handler doesn't know about
    return null;
  };
}
