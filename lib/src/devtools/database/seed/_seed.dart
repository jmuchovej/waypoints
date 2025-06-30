import 'package:uuid/uuid.dart';

import '../../../editor/editor.dart';
import '../../../things/task/task_entity.dart';
import '../seed.dart';

abstract class DatabaseSeed {
  void seed(SeedDataService service);
}

TaskEntity buildProjectScopeTask() => DevtoolsTaskDefaults.withDefaults(
  Uuid().v4(),
  AttributedText('Define project scope and objectives'),
  priority: TaskPriority.H,
  dateDelta: const Duration(days: 7),
);

TaskEntity buildProjectTimelineTask() => DevtoolsTaskDefaults.withDefaults(
  Uuid().v4(),
  AttributedText('Create project timeline'),
  dateDelta: const Duration(days: 14),
);

extension DevtoolsTaskDefaults on TaskEntity {
  static TaskEntity withDefaults(
    String uuid,
    AttributedText title, {
    DateTime? dueDate,
    Duration? dateDelta,
    TaskStatus status = TaskStatus.todo,
    TaskPriority priority = TaskPriority.M,
    TaskEnergy energy = TaskEnergy.M,
    TaskTime time = TaskTime.M,
    String? recurrence,
  }) {
    final now = DateTime.now();
    dueDate ??= dateDelta != null ? now.add(dateDelta) : now;

    final task = TaskEntity(
      uuid: uuid,
      title: title,
      dueDate: dueDate,
      status: status,
      priority: priority,
      // Simulate creation date
      createDate: now.subtract(Duration(days: 7)),
      // Simulate modification date (based on the due-date)
      modifyDate: now.subtract(
        Duration(hours: (now.difference(dueDate).inHours.abs() * 0.3).round()),
      ),
      energy: energy,
      time: time,
      recurrence: recurrence,
    );

    switch (status) {
      case TaskStatus.scheduled:
        task.scheduledDate = dueDate.subtract(const Duration(days: 1));
        break;
      case TaskStatus.started:
      case TaskStatus.workInProgress:
        task.startDate = now.subtract(const Duration(days: 2));
        break;
      case TaskStatus.complete:
        task.startDate = now.subtract(const Duration(days: 3));
        task.endDate = now.subtract(const Duration(days: 1));
        break;
      case TaskStatus.waiting:
        task.waitDate = now;
        task.untilDate = now.add(const Duration(days: 3));
        break;
      default:
        break;
    }

    return task;
  }
}
