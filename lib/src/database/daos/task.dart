import 'package:tempo/src/database/daos/_base.dart';
import 'package:tempo/src/database/schemas/task.dart';

class TaskDAO extends ThingDAO {
  TaskDAO(super.thing, super.schema);

  TaskStatus get status {
    final status = getProperty<String>('status');
    return TaskStatus.values.firstWhere(
      (s) => s.name == status,
      orElse: () => TaskStatus.todo,
    );
  }
  set status(TaskStatus status) => setProperty('status', status.name);

  TaskPriority get priority {
    final priority = getProperty<String>('priority');
    return TaskPriority.values.firstWhere(
      (p) => p.name == priority,
      orElse: () => TaskPriority.M,
    );
  }
  set priority(TaskPriority priority) => setProperty('priority', priority.name);

  TaskTimeEstimate get timeEstimate {
    final estimate = getProperty<String>('time_estimate');
    return TaskTimeEstimate.values.firstWhere(
      (t) => t.name == estimate,
      orElse: () => TaskTimeEstimate.M,
    );
  }
  set timeEstimate(TaskTimeEstimate estimate) => setProperty('time_estimate', estimate.name);

  TaskEnergyEstimate get energyEstimate {
    final estimate = getProperty<String>('energy_estimate');
    return TaskEnergyEstimate.values.firstWhere(
      (e) => e.name == estimate,
      orElse: () => TaskEnergyEstimate.M,
    );
  }
  set energyEstimate(TaskEnergyEstimate estimate) => setProperty('energy_estimate', estimate.name);

  // Date properties
  DateTime? get dueDate => getProperty<DateTime>('due_date');
  set dueDate(DateTime? date) => setProperty('due_date', date);

  DateTime? get startDate => getProperty<DateTime>('start_date');
  set startDate(DateTime? date) => setProperty('start_date', date);

  DateTime? get endDate => getProperty<DateTime>('end_date');
  set endDate(DateTime? date) => setProperty('end_date', date);

  DateTime? get scheduledDate => getProperty<DateTime>('scheduled_date');
  set scheduledDate(DateTime? date) => setProperty('scheduled_date', date);

  // Relationships
  String? get projectUuid => getReference('project');
  set projectUuid(String? uuid) => setReference('project', uuid);

  String? get parentUuid => getReference('parent');
  set parentUuid(String? uuid) => setReference('parent', uuid);

  List<String> get childrenUuids => getReferences('children');
  set childrenUuids(List<String> uuids) => setReferences('children', uuids);

  // Computed properties
  bool get isCompleted => status.isCompleted;
  bool get isOverdue => dueDate != null && dueDate!.isBefore(DateTime.now()) && !isCompleted;
}
