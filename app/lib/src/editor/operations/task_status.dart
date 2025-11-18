import '../listeners/autosave_listener.dart';
import '../super_editor.dart';
import '../../infrastructure/logging.dart';
import '../nodes/task_node.dart';
import '../../things/task/task.entity.dart';

/// Request to toggle a task's completion status
class ChangeTaskStatusRequest implements EditRequest {
  const ChangeTaskStatusRequest({required this.nodeID, required this.status});

  /// The ID of the TaskNode to toggle
  final String nodeID;
  final TaskStatus status;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ChangeTaskStatusRequest &&
          runtimeType == other.runtimeType &&
          nodeID == other.nodeID;

  @override
  int get hashCode => nodeID.hashCode;

  @override
  String toString() => 'ToggleTaskStatusRequest(nodeID: $nodeID)';
}

/// Command to toggle a task's completion status in the document
class ChangeTaskStatusCommand implements EditCommand {
  const ChangeTaskStatusCommand({required this.nodeID, required this.status});

  final String nodeID;
  final TaskStatus status;

  @override
  HistoryBehavior get historyBehavior => HistoryBehavior.undoable;

  @override
  String describe() => 'Toggle task status for node $nodeID';

  @override
  void execute(EditContext editContext, CommandExecutor executor) {
    final node = editContext.document.getNodeById(nodeID);

    if (node is! TaskNode) {
      waypointsTaskEditorLog.warning(
        'ToggleTaskStatusCommand: Node $nodeID is not a TaskNode',
      );
      return;
    }

    // Toggle the task status
    final currTask = node.task;
    final nextTask = currTask.copyWith(status: status);

    editContext.document.replaceNodeById(
      nodeID,
      node.copyTaskWith(task: nextTask),
    );

    executor.logChanges([DocumentEdit(ThingChangeEvent(currTask, nextTask))]);
  }

  @override
  String toString() => 'ToggleTaskStatusCommand(nodeId: $nodeID)';
}

EditCommand? changeTaskStatusHandler(Editor editor, EditRequest request) {
  if (request is! ChangeTaskStatusRequest) {
    return null;
  }

  return ChangeTaskStatusCommand(
    nodeID: request.nodeID,
    status: request.status,
  );
}
