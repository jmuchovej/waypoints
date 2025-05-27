import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/database/objectbox/models/task.dart';
import 'package:tempo/src/database/objectbox.g.dart';

/// Repository for managing TaskEntity operations with ObjectBox
class TaskRepository {
  TaskRepository(Store store) : _box = store.box<TaskEntity>();

  final Box<TaskEntity> _box;

  /// Create a new task
  int createTask({
    required String uuid,
    required AttributedText title,
    DateTime? dueDate,
    TaskStatus status = TaskStatus.todo,
    TaskPriority priority = TaskPriority.M,
    TaskTimeEstimate timeEstimate = TaskTimeEstimate.M,
    TaskEnergyEstimate energyEstimate = TaskEnergyEstimate.M,
  }) {
    final task = TaskEntity(
      uuid: uuid,
      title: title,
      dueDate: dueDate,
      status: status,
      priority: priority,
      timeEstimate: timeEstimate,
      energyEstimate: energyEstimate,
    );

    return _box.put(task);
  }

  /// Get all tasks
  List<TaskEntity> getAllTasks() {
    return _box.getAll();
  }

  /// Get task by ID
  TaskEntity? getTask(int id) {
    return _box.get(id);
  }

  /// Search tasks by title (plain text)
  List<TaskEntity> searchByTitle(String query) {
    final queryBuilder = _box.query(
      TaskEntity_.titlePlainText.contains(query, caseSensitive: false),
    );

    return queryBuilder.build().find();
  }

  /// Get tasks due today
  List<TaskEntity> getTasksDueToday() {
    final now = DateTime.now();
    final startOfDay = DateTime(now.year, now.month, now.day);
    final endOfDay = startOfDay.add(Duration(days: 1));

    final queryBuilder = _box.query(
      TaskEntity_.dueDate.between(
        startOfDay.millisecondsSinceEpoch,
        endOfDay.millisecondsSinceEpoch,
      ),
    );

    return queryBuilder.build().find();
  }

  /// Get inbox tasks (no project assigned)
  List<TaskEntity> getInboxTasks() {
    final queryBuilder = _box.query(
      TaskEntity_.project.isNull() &
          TaskEntity_.deleted.equals(false) &
          TaskEntity_.archived.equals(false),
    );

    return queryBuilder.build().find();
  }

  /// Update task title
  void updateTaskTitle(int taskId, AttributedText newTitle) {
    final task = _box.get(taskId);
    if (task != null) {
      task.title = newTitle;
      task.update(); // Updates modifyDate
      _box.put(task);
    }
  }

  /// Delete task (soft delete)
  void deleteTask(int taskId) {
    final task = _box.get(taskId);
    if (task != null) {
      task.deleted = true;
      task.update();
      _box.put(task);
    }
  }

  /// Archive task
  void archiveTask(int taskId) {
    final task = _box.get(taskId);
    if (task != null) {
      task.archived = true;
      task.update();
      _box.put(task);
    }
  }

  /// Calculate task urgency (TaskWarrior formula)
  double calculateUrgency(TaskEntity task) {
    double urgency = 0.0;

    // Priority component
    switch (task.priority) {
      case TaskPriority.H:
        urgency += 6.0;
        break;
      case TaskPriority.M:
        urgency += 3.9;
        break;
      case TaskPriority.L:
        urgency += 1.8;
        break;
    }

    // Due date component
    if (task.dueDate != null) {
      final daysUntilDue = task.dueDate.difference(DateTime.now()).inDays;
      if (daysUntilDue <= 0) {
        urgency += 12.0; // Overdue
      } else if (daysUntilDue <= 1) {
        urgency += 10.0;
      } else if (daysUntilDue <= 3) {
        urgency += 8.0;
      } else if (daysUntilDue <= 7) {
        urgency += 5.0;
      } else if (daysUntilDue <= 14) {
        urgency += 3.0;
      } else if (daysUntilDue <= 30) {
        urgency += 1.0;
      }
    }

    // Age component (older tasks get slightly higher urgency)
    final age = DateTime.now().difference(task.createDate).inDays;
    urgency += (age / 365.0) * 2.0; // Max 2 points for a year-old task

    // Status component
    if (task.status == TaskStatus.started) {
      urgency += 2.0;
    } else if (task.status == TaskStatus.waiting) {
      urgency -= 3.0;
    }

    return urgency;
  }

  /// Get tasks sorted by urgency
  List<TaskEntity> getTasksByUrgency() {
    final tasks =
        _box
            .query(
              TaskEntity_.deleted.equals(false) &
                  TaskEntity_.archived.equals(false) &
                  TaskEntity_.status.notOneOf([
                    TaskStatus.complete.index,
                    TaskStatus.dropped.index,
                  ]),
            )
            .build()
            .find();

    // Sort by urgency (descending)
    tasks.sort((a, b) => calculateUrgency(b).compareTo(calculateUrgency(a)));

    return tasks;
  }
}
