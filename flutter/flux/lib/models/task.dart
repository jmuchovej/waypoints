import 'package:objectbox/objectbox.dart';
import 'task/_status.dart' hide assertStableEnumValues;
import 'task/_status.dart' as S show assertStableEnumValues;
import 'task/_energy.dart' hide assertStableEnumValues;
import 'task/_energy.dart' as E show assertStableEnumValues;
import 'task/_priority.dart' hide assertStableEnumValues;
import 'task/_priority.dart' as P show assertStableEnumValues;
import 'task/_time.dart' hide assertStableEnumValues;
import 'task/_time.dart' as T show assertStableEnumValues;
import 'task_list.dart';

export 'task/_status.dart' hide assertStableEnumValues;
export 'task/_priority.dart' hide assertStableEnumValues;
export 'task/_energy.dart' hide assertStableEnumValues;
export 'task/_time.dart' hide assertStableEnumValues;

@Entity()
class Task {
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
  TaskTimeEstimate estimatedTime;

  @Transient()
  TaskEnergyEstimate estimatedEnergy;

  @Transient()
  TaskPriority priority;

  bool deleted;
  bool archived;

  final parent = ToOne<Task>();
  final children = ToMany<Task>();
  final taskList = ToOne<TaskList>();

  Task({
    required this.uuid,
    required this.name,
    required this.description,
    required this.createDate,
    required this.modifyDate,
    required this.dueDate,
    this.startDate,
    this.endDate,
    this.scheduledDate,
    this.waitDate,
    this.untilDate,
    this.recurrence,
    this.archived = false,
    this.deleted = false,
    this.status = TaskStatus.todo,
    this.estimatedTime = TaskTimeEstimate.M,
    this.estimatedEnergy = TaskEnergyEstimate.M,
    this.priority = TaskPriority.M,
  });

  void update() {
    modifyDate = DateTime.now();
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! Task) return false;
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
    return estimatedEnergy.index;
  }

  set dbEstimatedEnergy(int value) {
    estimatedEnergy = TaskEnergyEstimate.values[value];
  }

  int get dbEstimatedTime {
    T.assertStableEnumValues();
    return estimatedTime.index;
  }

  set dbEstimatedTime(int value) {
    estimatedTime = TaskTimeEstimate.values[value];
  }

  int get dbPriority {
    P.assertStableEnumValues();
    return priority.index;
  }

  set dbPriority(int value) {
    priority = TaskPriority.values[value];
  }
}
