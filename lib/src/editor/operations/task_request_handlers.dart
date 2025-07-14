import '../super_editor.dart';
import '../../things/task/task_repository.dart';
import 'task_requests.dart';
import 'task_commands.dart';

/// Creates a task request handler with access to the task repository
///
/// This function returns a handler closure that captures the TaskRepository,
/// allowing the handler to create commands that can persist changes to the database.
EditRequestHandler createTaskRequestHandler(TaskRepository taskRepository) {
  return (Editor editor, EditRequest request) {
    if (request is ToggleTaskStatusRequest) {
      return ToggleTaskStatusCommand(
        nodeId: request.nodeId,
        taskRepository: taskRepository,
      );
    }

    // Return null for requests this handler doesn't know about
    return null;
  };
}
