import '../super_editor.dart';
import '../../infrastructure/logging.dart';
import '../nodes/task_node.dart';
import '../../things/task/task_entity.dart';
import '../../things/task/task_repository.dart';

/// Command to toggle a task's completion status in the document
class ToggleTaskStatusCommand implements EditCommand {
  const ToggleTaskStatusCommand({
    required this.nodeId,
    required this.taskRepository,
  });

  final String nodeId;
  final TaskRepository taskRepository;

  @override
  HistoryBehavior get historyBehavior => HistoryBehavior.undoable;

  @override
  String describe() => 'Toggle task status for node $nodeId';

  @override
  void execute(EditContext editContext, CommandExecutor executor) {
    final document = editContext.find<MutableDocument>(Editor.documentKey);
    if (document == null) {
      fluxLog.warning('ToggleTaskStatusCommand: No document found in edit context');
      return;
    }

    final node = document.getNodeById(nodeId);
    if (node is! TaskNode) {
      fluxLog.warning('ToggleTaskStatusCommand: Node $nodeId is not a TaskNode');
      return;
    }

    // Toggle the task status
    final currentStatus = node.task.status;
    final newStatus = currentStatus == TaskStatus.complete
        ? TaskStatus.todo
        : TaskStatus.complete;

    fluxLog.info('ToggleTaskStatusCommand: Optimistically updating task ${node.task.uuid} from $currentStatus to $newStatus');

    // OPTIMISTIC UPDATE: Update UI immediately
    final optimisticTask = node.task.copyWith(status: newStatus);
    final optimisticNode = node.copyTaskWith(task: optimisticTask);
    document.replaceNodeById(node.id, optimisticNode);
    fluxLog.info('ToggleTaskStatusCommand: UI updated optimistically for task ${node.task.uuid}');

    // ASYNC DATABASE UPDATE: Update database in background
    _updateDatabaseAsync(node.task.uuid, currentStatus, newStatus, document, node);
  }

  /// Updates the database asynchronously and handles rollback on failure
  void _updateDatabaseAsync(
    String taskUuid,
    TaskStatus originalStatus,
    TaskStatus newStatus,
    MutableDocument document,
    TaskNode originalNode,
  ) async {
    try {
      // Update the database
      taskRepository.updateStatus(taskUuid, newStatus);
      fluxLog.info('ToggleTaskStatusCommand: Database successfully updated for task $taskUuid');

      // Verify the update worked
      final verifiedTask = taskRepository.byUUID(taskUuid);
      if (verifiedTask?.status != newStatus) {
        throw Exception('Database update verification failed: expected $newStatus, got ${verifiedTask?.status}');
      }
    } catch (error, stackTrace) {
      fluxLog.severe('ToggleTaskStatusCommand: Database update failed for task $taskUuid', error, stackTrace);

      // ROLLBACK: Revert the optimistic update
      final rollbackTask = originalNode.task.copyWith(status: originalStatus);
      final rollbackNode = originalNode.copyTaskWith(task: rollbackTask);
      document.replaceNodeById(originalNode.id, rollbackNode);
      fluxLog.warning('ToggleTaskStatusCommand: Rolled back optimistic update for task $taskUuid');
    }
  }

  @override
  String toString() => 'ToggleTaskStatusCommand(nodeId: $nodeId)';
}
