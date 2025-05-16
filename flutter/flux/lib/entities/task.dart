import 'package:objectbox/objectbox.dart';
import 'task/_status.dart' hide assertStableEnumValues;
import 'task/_status.dart' as S show assertStableEnumValues;
import 'task/_energy.dart' hide assertStableEnumValues;
import 'task/_energy.dart' as E show assertStableEnumValues;
import 'task/_priority.dart' hide assertStableEnumValues;
import 'task/_priority.dart' as P show assertStableEnumValues;
import 'task/_time.dart' hide assertStableEnumValues;
import 'task/_time.dart' as T show assertStableEnumValues;
import 'project.dart';

export 'task/_status.dart' hide assertStableEnumValues;
export 'task/_priority.dart' hide assertStableEnumValues;
export 'task/_energy.dart' hide assertStableEnumValues;
export 'task/_time.dart' hide assertStableEnumValues;

@Entity()
class TaskEntity {
  @Id()
  int id = 0;

  String uuid;
  String name;
  String description;

  @Property(type: PropertyType.date)
  DateTime createDate;

  @Property(type: PropertyType.date)
  DateTime modifyDate;

  @Property(type: PropertyType.date)
  DateTime? startDate;

  @Property(type: PropertyType.date)
  DateTime? endDate;

  @Property(type: PropertyType.date)
  DateTime? scheduledDate;

  @Property(type: PropertyType.date)
  DateTime dueDate;

  @Property(type: PropertyType.date)
  DateTime? waitDate;

  @Property(type: PropertyType.date)
  DateTime? untilDate;

  String? recurrence;

  @Transient()
  TaskStatus status;

  @Transient()
  TaskTimeEstimate timeEstimate;

  @Transient()
  TaskEnergyEstimate energyEstimate;

  @Transient()
  TaskPriority priority;

  bool deleted;
  bool archived;

  final parent = ToOne<TaskEntity>();
  @Backlink()
  final children = ToMany<TaskEntity>();

  final project = ToOne<ProjectEntity>();

  TaskEntity({
    required this.uuid,
    required this.name,
    DateTime? createDate,
    DateTime? modifyDate,
    DateTime? dueDate,
    this.description = "",
    this.startDate,
    this.endDate,
    this.scheduledDate,
    this.waitDate,
    this.untilDate,
    this.recurrence,
    this.archived = false,
    this.deleted = false,
    this.status = TaskStatus.todo,
    this.timeEstimate = TaskTimeEstimate.M,
    this.energyEstimate = TaskEnergyEstimate.M,
    this.priority = TaskPriority.M,
  }) : dueDate = dueDate ?? DateTime.now().add(Duration(days: 1)),
       createDate = createDate ?? DateTime.now(),
       modifyDate = modifyDate ?? DateTime.now();

  TaskEntity copyWith({
    String? uuid,
    String? name,
    String? description,
    DateTime? createDate,
    DateTime? modifyDate,
    DateTime? startDate,
    DateTime? endDate,
    DateTime? scheduledDate,
    DateTime? dueDate,
    DateTime? waitDate,
    DateTime? untilDate,
    String? recurrence,
    TaskStatus? status,
    TaskTimeEstimate? timeEstimate,
    TaskEnergyEstimate? energyEstimate,
    TaskPriority? priority,
    bool? deleted,
    bool? archived,
    TaskEntity? parent,
    List<TaskEntity>? children,
    ProjectEntity? project,
  }) {
    final task = TaskEntity(
      uuid: uuid ?? this.uuid,
      name: name ?? this.name,
      description: description ?? this.description,
      createDate: createDate ?? this.createDate,
      modifyDate: modifyDate ?? this.modifyDate,
      startDate: startDate ?? this.startDate,
      endDate: endDate ?? this.endDate,
      scheduledDate: scheduledDate ?? this.scheduledDate,
      dueDate: dueDate ?? this.dueDate,
      waitDate: waitDate ?? this.waitDate,
      untilDate: untilDate ?? this.untilDate,
      recurrence: recurrence ?? this.recurrence,
      status: status ?? this.status,
      timeEstimate: timeEstimate ?? this.timeEstimate,
      energyEstimate: energyEstimate ?? this.energyEstimate,
      priority: priority ?? this.priority,
      deleted: deleted ?? this.deleted,
      archived: archived ?? this.archived,
    );

    task.parent.target = parent ?? this.parent.target;
    for (final child in (children ?? this.children.toList())) {
      task.children.add(child);
    }
    task.project.target = project ?? this.project.target;

    return task;
  }

  void update() {
    modifyDate = DateTime.now();
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! TaskEntity) return false;
    return id == other.id && uuid == other.uuid;
  }

  int get dbStatus {
    S.assertStableEnumValues();
    return status.index;
  }

  set dbStatus(int value) {
    status = TaskStatus.values[value];
  }

  int get dbEstimatedEnergy {
    E.assertStableEnumValues();
    return energyEstimate.index;
  }

  set dbEstimatedEnergy(int value) {
    energyEstimate = TaskEnergyEstimate.values[value];
  }

  int get dbEstimatedTime {
    T.assertStableEnumValues();
    return timeEstimate.index;
  }

  set dbEstimatedTime(int value) {
    timeEstimate = TaskTimeEstimate.values[value];
  }

  int get dbPriority {
    P.assertStableEnumValues();
    return priority.index;
  }

  set dbPriority(int value) {
    priority = TaskPriority.values[value];
  }
}
