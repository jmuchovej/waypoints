import 'package:objectbox/objectbox.dart';
import './task/status.dart' hide assertStableEnumValues;
import './task/status.dart' as S show assertStableEnumValues;
import './task/energy.dart' hide assertStableEnumValues;
import './task/energy.dart' as E show assertStableEnumValues;
import './task/priority.dart' hide assertStableEnumValues;
import './task/priority.dart' as P show assertStableEnumValues;
import './task/time.dart' hide assertStableEnumValues;
import './task/time.dart' as T show assertStableEnumValues;
import 'task_list.dart';

export './task/status.dart' hide assertStableEnumValues;
export './task/priority.dart' hide assertStableEnumValues;
export './task/energy.dart' hide assertStableEnumValues;
export './task/time.dart' hide assertStableEnumValues;

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
  Status status;

  @Transient()
  EstimatedTime estimatedTime;

  @Transient()
  EstimatedEnergy estimatedEnergy;

  @Transient()
  Priority priority;

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
    this.status = Status.todo,
    this.estimatedTime = EstimatedTime.unknown,
    this.estimatedEnergy = EstimatedEnergy.unknown,
    this.priority = Priority.unknown,
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

  set dbStatus(int? value) {
    if (value == null) {
      status = Status.unknown;
      return;
    }

    status =
        value > 0 && value < Status.values.length
            ? Status.values[value]
            : Status.unknown;
  }

  int get dbEstimatedEnergy {
    E.assertStableEnumValues();
    return estimatedEnergy.index;
  }

  set dbEstimatedEnergy(int? value) {
    if (value == null) {
      estimatedEnergy = EstimatedEnergy.unknown;
      return;
    }

    estimatedEnergy =
        value > 0 && value < EstimatedEnergy.values.length
            ? EstimatedEnergy.values[value]
            : EstimatedEnergy.unknown;
  }

  int get dbEstimatedTime {
    T.assertStableEnumValues();
    return estimatedTime.index;
  }

  set dbEstimatedTime(int? value) {
    if (value == null) {
      estimatedTime = EstimatedTime.unknown;
      return;
    }

    estimatedTime =
        value > 0 && value < EstimatedTime.values.length
            ? EstimatedTime.values[value]
            : EstimatedTime.unknown;
  }

  int get dbPriority {
    P.assertStableEnumValues();
    return priority.index;
  }

  set dbPriority(int? value) {
    if (value == null) {
      priority = Priority.unknown;
      return;
    }

    priority =
        value > 0 && value < Priority.values.length
            ? Priority.values[value]
            : Priority.unknown;
  }
}
