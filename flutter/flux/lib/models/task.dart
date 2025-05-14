import 'package:objectbox/objectbox.dart';
import 'task_list.dart';

enum Status {
  unknown, // Safety fallback
  todo, // Initial state
  started, // We've started but not focused on it
  waiting, // Blocked by something
  paused, // Interrupted but will resume
  workInProgress, // Parent task with child being worked on
  scheduled, // Future task
  stopped, // No longer working on it
  dropped, // Decided not to complete
  complete, // Finished! 🎉
  archived, // Hidden but retrievable
  deleted, // Marked as deleted but not removed
}

extension StatusGroups on Status {
  bool get isCompleted {
    return [
      Status.dropped,
      Status.complete,
      Status.archived,
      Status.deleted,
    ].contains(this);
  }

  bool get isPending {
    return [
      Status.paused,
      Status.workInProgress,
      Status.scheduled,
      Status.stopped,
    ].contains(this);
  }

  bool get isAwaiting {
    return [Status.todo, Status.started, Status.waiting].contains(this);
  }
}

void _assertStableEnumValuesStatus() {
  assert(0 == Status.unknown.index);
  assert(1 == Status.todo.index);
  assert(2 == Status.started.index);
  assert(3 == Status.waiting.index);
  assert(4 == Status.paused.index);
  assert(5 == Status.workInProgress.index);
  assert(6 == Status.scheduled.index);
  assert(7 == Status.stopped.index);
  assert(8 == Status.dropped.index);
  assert(9 == Status.complete.index);
  assert(10 == Status.archived.index);
  assert(11 == Status.deleted.index);
}

enum EstimatedTime {
  unknown, // Safety fallback
  XS, // Extra small - almost automatic
  S, // Small - quick task
  M, // Medium - average task
  L, // Large - significant time
  XL, // Extra large - major time commitment
}

void _assertStableEnumValuesTime() {
  assert(0 == EstimatedTime.unknown.index);
  assert(1 == EstimatedTime.XS.index);
  assert(2 == EstimatedTime.S.index);
  assert(3 == EstimatedTime.M.index);
  assert(4 == EstimatedTime.L.index);
  assert(5 == EstimatedTime.XL.index);
}

enum EstimatedEnergy {
  unknown, // Safety fallback
  XS, // Almost no energy
  S, // Little energy
  M, // Medium energy
  L, // Significant energy
  XL, // Peak energy required
}

void _assertStableEnumValuesEnergy() {
  assert(0 == EstimatedEnergy.unknown.index);
  assert(1 == EstimatedEnergy.XS.index);
  assert(2 == EstimatedEnergy.S.index);
  assert(3 == EstimatedEnergy.M.index);
  assert(4 == EstimatedEnergy.L.index);
  assert(5 == EstimatedEnergy.XL.index);
}

enum Priority {
  unknown, // Safety fallback
  L, // Low priority
  M, // Medium priority
  H, // High priority
}

void _assertStableEnumValuesPriority() {
  assert(0 == Priority.unknown.index);
  assert(1 == Priority.L.index);
  assert(2 == Priority.M.index);
  assert(3 == Priority.H.index);
}

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
  EstimatedTime? estimatedTime;

  @Transient()
  EstimatedEnergy? estimatedEnergy;

  @Transient()
  Priority? priority;

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
    this.status = Status.todo,
    EstimatedTime? estimatedTime,
    EstimatedEnergy? estimatedEnergy,
    Priority? priority,
  }) : estimatedTime = estimatedTime ?? EstimatedTime.unknown,
       estimatedEnergy = estimatedEnergy ?? EstimatedEnergy.unknown,
       priority = priority ?? Priority.unknown;

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! Task) return false;
    return id == other.id && uuid == other.uuid;
  }

  int get dbStatus {
    _assertStableEnumValuesStatus();
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
    _assertStableEnumValuesEnergy();
    return (estimatedEnergy ?? EstimatedEnergy.unknown).index;
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
    _assertStableEnumValuesTime();
    return (estimatedTime ?? EstimatedTime.unknown).index;
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
    _assertStableEnumValuesPriority();
    return (priority ?? Priority.unknown).index;
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
