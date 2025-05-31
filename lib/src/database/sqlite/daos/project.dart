import 'package:tempo/src/database/sqlite/daos/_base.dart';
import 'package:tempo/src/database/sqlite/schemas/project.dart';
import 'package:tempo/src/database/sqlite/schemas/task.dart';

class ProjectDAO extends ThingDAO {
  ProjectDAO(super.thing, super.schema);

  String get description => getProperty<String>('description') ?? '';
  set description(String description) => setProperty('description', description);

  ProjectStatus get status {
    final status = getProperty<String>('status');
    return ProjectStatus.values.firstWhere(
      (s) => s.name == status,
      orElse: () => ProjectStatus.planning,
    );
  }
  set status(ProjectStatus status) => setProperty('status', status.name);

  TaskPriority get priority {
    final priority = getProperty<String>('priority');
    return TaskPriority.values.firstWhere(
      (p) => p.name == priority,
      orElse: () => TaskPriority.M,
    );
  }
  set priority(TaskPriority priority) => setProperty('priority', priority.name);

  // Date properties
  DateTime? get dueDate => getProperty<DateTime>('due_date');
  set dueDate(DateTime? date) => setProperty('due_date', date);

  DateTime? get startDate => getProperty<DateTime>('start_date');
  set startDate(DateTime? date) => setProperty('start_date', date);

  DateTime? get endDate => getProperty<DateTime>('end_date');
  set endDate(DateTime? date) => setProperty('end_date', date);

  // Relationships
  String? get parentProject => getReference('parent');
  set parentProject(String? uuid) => setReference('parent', uuid);

  List<String> get subProjects => getReferences('children');
  set subProjects(List<String> uuids) => setReferences('children', uuids);

  List<String> get tasks => getReferences('tasks');
  set tasks(List<String> uuids) => setReferences('tasks', uuids);

  // Computed properties
  bool get isCompleted => status.isCompleted;
  bool get isOverdue => dueDate != null && dueDate!.isBefore(DateTime.now()) && !isCompleted;
}
