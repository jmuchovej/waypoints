import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../objectbox/objectbox.g.dart';
import '../thing/interface.dart';
import 'task_entity.dart';

/// Provider for TaskRepository
final taskRepositoryProvider = Provider<TaskRepository>((ref) {
  return TaskRepository(ref);
});

/// Repository for TaskEntity with specific task queries
class TaskRepository extends ThingRepository<TaskEntity> {
  TaskRepository(super._ref);

  @override
  Box<TaskEntity> get box => Box<TaskEntity>(store);

  @override
  String get type => TaskEntity.key;

  /// Archive a task and its subtasks
  void archive(String taskUuid) {
    final task = getByUUID(taskUuid);
    if (task == null) return;

    final updatedTask = task.copyWith(archived: true);
    put(updatedTask);

    final subtasks = sameTypedChildren(taskUuid);
    for (final subtask in subtasks) {
      archive(subtask.uuid);
    }
  }

  /// Complete a task and optionally complete its subtasks
  void complete(String taskUuid, {bool includeSubtasks = false}) {
    final task = getByUUID(taskUuid);
    if (task == null) return;

    final updatedTask = task.copyWith(status: TaskStatus.complete);
    put(updatedTask);

    if (includeSubtasks) {
      final subtasks = sameTypedChildren(taskUuid);
      for (final subtask in subtasks) {
        complete(subtask.uuid, includeSubtasks: false);
      }
    }
  }

  /// Get all tasks due today
  List<TaskEntity> dueToday() {
    final today = DateTime.now();
    final startOfDay =
        DateTime(today.year, today.month, today.day).millisecondsSinceEpoch;
    final endOfDay =
        DateTime(today.year, today.month, today.day + 1).millisecondsSinceEpoch;

    final query = box.query(TaskEntity_.dueDate.between(startOfDay, endOfDay));
    return query.build().find();
  }

  /// Get all incomplete tasks (application-layer filtering)
  List<TaskEntity> incomplete() {
    // Use application logic for the isCompleted business rule
    return getAll().where((task) => !task.status.isCompleted).toList();
  }

  /// Get all tasks in a specific project (including nested in subtasks)
  List<TaskEntity> inProjectDeep(String projectUuid) {
    final directTasks = sameTypedChildren(projectUuid);
    final allTasks = <TaskEntity>[...directTasks];

    // Recursively get tasks from subtasks
    void collectSubtasks(TaskEntity parent) {
      final subtasks = sameTypedChildren(parent.uuid);
      allTasks.addAll(subtasks);
      for (final subtask in subtasks) {
        collectSubtasks(subtask);
      }
    }

    for (final task in directTasks) {
      collectSubtasks(task);
    }

    return allTasks;
  }

  /// Get all overdue tasks (application-layer filtering for business logic)
  List<TaskEntity> overdue() {
    final now = DateTime.now();
    // Use ObjectBox for date filtering, then application logic for status
    final query = box.query(
      TaskEntity_.dueDate.lessThan(now.millisecondsSinceEpoch),
    );
    return query
        .build()
        .find()
        .where((task) => !task.status.isCompleted)
        .toList();
  }

  /// Get tasks scheduled for a specific date range
  List<TaskEntity> scheduledBetween(DateTime start, DateTime end) {
    final query = box.query(
      TaskEntity_.scheduledDate.between(
        start.millisecondsSinceEpoch,
        end.millisecondsSinceEpoch,
      ),
    );
    return query.build().find();
  }

  /// Unarchive a task
  void unarchive(String taskUuid) {
    final task = getByUUID(taskUuid);
    if (task == null) return;

    final updatedTask = task.copyWith(archived: false);
    put(updatedTask);
  }

  /// Update task priority
  void updatePriority(String taskUuid, TaskPriority priority) {
    final task = getByUUID(taskUuid);
    if (task == null) return;

    final updatedTask = task.copyWith(priority: priority);
    put(updatedTask);
  }

  /// Update task status
  void updateStatus(String taskUuid, TaskStatus status) {
    final task = getByUUID(taskUuid);
    if (task == null) return;

    final updatedTask = task.copyWith(status: status);
    put(updatedTask);
  }

  /// Get tasks waiting until a specific date
  List<TaskEntity> waitingUntil(DateTime date) {
    final query = box.query(
      TaskEntity_.waitDate.lessThan(date.millisecondsSinceEpoch),
    );
    return query.build().find();
  }

  /// Get tasks by priority
  List<TaskEntity> withPriority(TaskPriority priority) {
    final query = box.query(TaskEntity_.obxPriority.equals(priority.index));
    return query.build().find();
  }

  /// Get all tasks with a specific status
  List<TaskEntity> withStatus(TaskStatus status) {
    final query = box.query(TaskEntity_.obxStatus.equals(status.index));
    return query.build().find();
  }
}
