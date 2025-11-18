import '../super_editor.dart';

/// Request to open a task panel for a specific task
class OpenTaskPanelRequest implements EditRequest {
  const OpenTaskPanelRequest({required this.taskUuid});

  final String taskUuid;
}

/// Request to open a project panel for a specific project
class OpenProjectPanelRequest implements EditRequest {
  const OpenProjectPanelRequest({required this.projectUuid});

  final String projectUuid;
}

/// Request to navigate to a specific route
class NavigateToRouteRequest implements EditRequest {
  const NavigateToRouteRequest({required this.routePath, this.parameters});

  final String routePath;
  final Map<String, String>? parameters;
}
