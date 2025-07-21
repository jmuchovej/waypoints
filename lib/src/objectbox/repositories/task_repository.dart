import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../infrastructure/logging.dart';
import '../objectbox.dart';
import '../objectbox.g.dart';
import 'thing_repository.dart';
import '../../things/project/project.entity.dart';
import '../../things/thing.entity.dart';
import '../../things/task/task.entity.dart';

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
  void archive(String uuid) {
    final task = getByUUID(uuid);
    if (task == null) return;

    final updatedTask = task.copyWith(archived: true);
    put(updatedTask);

    final subtasks = sameTypedChildren(uuid);
    for (final subtask in subtasks) {
      archive(subtask.uuid);
    }
  }

  /// Complete a task and optionally complete its subtasks
  void complete(String uuid, {bool includeSubtasks = false}) {
    final task = getByUUID(uuid);
    if (task == null) return;

    final updatedTask = task.copyWith(status: TaskStatus.complete);
    put(updatedTask);

    if (includeSubtasks) {
      final subtasks = sameTypedChildren(uuid);
      for (final subtask in subtasks) {
        complete(subtask.uuid, includeSubtasks: false);
      }
    }
  }

  /// Get all tasks due today
  Condition<TaskEntity> dueToday() {
    final t = DateTime.now();
    final s = DateTime(t.year, t.month, t.day);
    final e = DateTime(t.year, t.month, t.day + 1);

    return dateBetween(TaskOBX.dueDate, start: s, end: e);
  }

  /// Get all incomplete tasks (application-layer filtering)
  Condition<TaskEntity> incomplete() {
    return TaskStatusGroups.isCompleted;
  }

  /// Get all tasks in a specific project (including nested in subtasks)
  List<TaskEntity> inProjectDeep(String uuid) {
    final directTasks = sameTypedChildren(uuid);
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

  List<TaskEntity> immediateSubtasks(TaskEntity task) {
    final childThings = task.thing.children.where((e) => e.type == type);
    tempoDBLog.finest('... for `task=${task.uuid}`, found n-children=${childThings.length}');
    final childIDs = childThings.map((c) => c.id).toList();
    final results = box.getMany(childIDs);
    return results.where((subtask) => subtask != null).toList().cast<TaskEntity>();
  }

  ProjectEntity? immediateProject(TaskEntity task) {
    final parentThing = task.thing.parent;
    if (parentThing == null) return null;
    if (parentThing.type != ProjectEntity.key) return null;

    return store.box<ProjectEntity>().get(parentThing.id)!;
  }

  /// Get all overdue tasks (application-layer filtering for business logic)
  Condition<TaskEntity> overdue() {
    final now = DateTime.now();
    return TaskOBX.dueDate
        .lessThan(now.millisecondsSinceEpoch)
        .and(TaskStatusGroups.isCompleted.or(TaskStatusGroups.isAwaiting));
  }

  /// Get tasks scheduled for a specific date range
  Condition<TaskEntity> scheduledBetween(DateTime start, DateTime end) {
    return dateBetween(TaskOBX.scheduledDate, start: start, end: end);
  }

  /// Unarchive a task
  void unarchive(String uuid) {
    final task = getByUUID(uuid);
    if (task == null) return;

    final updatedTask = task.copyWith(archived: false);
    put(updatedTask);
  }

  /// Update task priority
  void updatePriority(String uuid, TaskPriority priority) {
    final task = getByUUID(uuid);
    if (task == null) return;

    final updatedTask = task.copyWith(priority: priority);
    put(updatedTask);
  }

  /// Update task status
  void updateStatus(String uuid, TaskStatus status) {
    final task = getByUUID(uuid);
    if (task == null) return;

    final updatedTask = task.copyWith(status: status);
    put(updatedTask);
  }

  /// Get tasks waiting until a specific date
  Condition<TaskEntity> waitingUntil(DateTime date) {
    return TaskOBX.waitDate.lessThan(date.millisecondsSinceEpoch);
  }

  /// Get all tasks with a specific status
  Condition<TaskEntity> withStatus(TaskStatus status) {
    return TaskOBX.statusOBX.equals(status.index);
  }

  /// Build a [Condition<TaskEntity>] based on the [TaskPriority].
  Condition<TaskEntity> withPriority(TaskPriority priority) {
    return TaskOBX.priorityOBX.equals(priority.index);
  }

  Condition<TaskEntity> archived(bool archived) {
    return TaskOBX.archived.equals(archived);
  }

  Condition<TaskEntity> deleted(bool archived) {
    return TaskOBX.deleted.equals(archived);
  }

  Condition<TaskEntity> active() {
    return archived(false).and(deleted(false));
  }

  /// Wrap any TaskEntity stream to filter for top-level tasks only
  /// This applies parent filtering after database-level filtering for efficiency
  Stream<List<TaskEntity>> topLevel(Stream<List<TaskEntity>> taskStream) {
    return taskStream.map((tasks) {
      tempoDBLog.info('topLevel: Filtering ${tasks.length} active tasks');
      final topLevelTasks =
          tasks.where((task) {
            if (task.parent == null) {
              // No parent = top-level task
              return true;
            }

            final parent = task.parent!;
            // Has parent - check if parent is a task (child task) or project (top-level task)
            final isChildOfTask = parent.type == TaskEntity.key;
            if (isChildOfTask) {
              tempoDBQueryLog.finer(
                'Filtering out subtask: ${task.title.toPlainText()} (parent task: ${parent.uuid})',
              );
              return false;
            }

            // Parent is project/other = top-level task
            tempoDBQueryLog.finest(
              'Top-level task: ${task.title.toPlainText()} (parent ${parent.type}: ${parent.uuid})',
            );
            return true;
          }).toList();

      tempoDBQueryLog.fine(
        'topLevel: Found ${topLevelTasks.length} top-level tasks',
      );
      return topLevelTasks;
    });
  }

  // ---- Streaming Extensions ----

  /// Stream all tasks due today
  Stream<List<TaskEntity>> streamDueToday() {
    return streamList(dueToday());
  }

  /// Stream all incomplete tasks
  Stream<List<TaskEntity>> streamIncomplete() {
    return streamList(incomplete());
  }

  /// Stream all overdue tasks
  Stream<List<TaskEntity>> streamOverdue() {
    return streamList(overdue());
  }

  /// Stream all active (non-archived, non-deleted) tasks
  Stream<List<TaskEntity>> streamActive() {
    return streamList(active());
  }

  /// Stream all top-level active tasks
  Stream<List<TaskEntity>> streamTopLevel() {
    tempoDBLog.info('streamTopLevel: Starting top-level task stream');
    return topLevel(streamActive());
  }

  /// Stream tasks with a specific status
  Stream<List<TaskEntity>> streamWithStatus(TaskStatus status) {
    return streamList(withStatus(status));
  }

  /// Stream tasks with a specific priority
  Stream<List<TaskEntity>> streamWithPriority(TaskPriority priority) {
    return streamList(withPriority(priority));
  }

  /// Stream task count for a condition (useful for badges/counters)
  Stream<int> streamCountDueToday() => streamCount(dueToday());
  Stream<int> streamCountOverdue() => streamCount(overdue());
  Stream<int> streamCountIncomplete() => streamCount(incomplete());
  Stream<int> streamCountTopLevel() =>
      streamTopLevel().map((tasks) => tasks.length);

  Condition<ThingEntity> searchTitle(String query) {
    return ThingOBX.type
        .equals(TaskEntity.key)
        .and(
          ThingOBX.plainText.contains(
            query.toLowerCase(),
            caseSensitive: false,
          ),
        );
  }
}
