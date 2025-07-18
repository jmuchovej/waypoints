import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../super_editor.dart';
import '../../infrastructure/logging.dart';
import '../../router.dart';

/// Command to open a task panel (this doesn't modify the document, but triggers navigation)
class OpenTaskPanelCommand implements EditCommand {
  const OpenTaskPanelCommand({
    required this.taskUuid,
    required this.context,
    required this.ref,
  });

  final String taskUuid;
  final BuildContext context;
  final WidgetRef ref;

  @override
  HistoryBehavior get historyBehavior => HistoryBehavior.undoable;

  @override
  String describe() => 'Open task panel for task $taskUuid';

  @override
  void execute(EditContext editContext, CommandExecutor executor) {
    tempoEditorLog.info(
      'OpenTaskPanelCommand: Opening panel for task $taskUuid',
    );

    // Navigate to the task using panel navigation
    context.pushThing(taskUuid);
  }

  @override
  String toString() => 'OpenTaskPanelCommand(taskUuid: $taskUuid)';
}

/// Command to open a project panel
class OpenProjectPanelCommand implements EditCommand {
  const OpenProjectPanelCommand({
    required this.projectUuid,
    required this.context,
    required this.ref,
  });

  final String projectUuid;
  final BuildContext context;
  final WidgetRef ref;

  @override
  HistoryBehavior get historyBehavior => HistoryBehavior.undoable;

  @override
  String describe() => 'Open project panel for project $projectUuid';

  @override
  void execute(EditContext editContext, CommandExecutor executor) {
    tempoEditorLog.info(
      'OpenProjectPanelCommand: Opening panel for project $projectUuid',
    );

    // Navigate to the project using panel navigation
    context.pushThing(projectUuid);
  }

  @override
  String toString() => 'OpenProjectPanelCommand(projectUuid: $projectUuid)';
}

/// Command to navigate to a specific route
class NavigateToRouteCommand implements EditCommand {
  const NavigateToRouteCommand({
    required this.routePath,
    required this.context,
    this.parameters,
  });

  final String routePath;
  final BuildContext context;
  final Map<String, String>? parameters;

  @override
  HistoryBehavior get historyBehavior => HistoryBehavior.undoable;

  @override
  String describe() => 'Navigate to route $routePath';

  @override
  void execute(EditContext editContext, CommandExecutor executor) {
    tempoEditorLog.info('NavigateToRouteCommand: Navigating to $routePath');

    // TODO: Implement navigation - this is a proof of concept
    // The navigation would go here, but needs proper route handling
    tempoEditorLog.info(
      'NavigateToRouteCommand: Would navigate to $routePath with params $parameters',
    );
  }

  @override
  String toString() => 'NavigateToRouteCommand(routePath: $routePath)';
}
