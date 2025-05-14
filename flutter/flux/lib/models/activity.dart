import 'package:objectbox/objectbox.dart';
import 'task.dart' show Task;

enum Kind {
  unknown, // Safety fallback
  activity, // Generic activity
  task, // Working on task
  event, // Calendar event
  intervalWork, // Work period (Pomodoro)
  intervalRest, // Rest period (Pomodoro)
  timeBlock, // Time-block with related tasks
}

void _assertStableEnumValuesKind() {
  assert(0 == Kind.unknown.index);
  assert(1 == Kind.activity.index);
  assert(2 == Kind.task.index);
  assert(3 == Kind.event.index);
  assert(4 == Kind.intervalWork.index);
  assert(5 == Kind.intervalRest.index);
  assert(6 == Kind.timeBlock.index);
}

enum Status {
  unknown, // Safety fallback
  created, // Initial state
  scheduled, // Start time assigned but not started
  inProgress, // Currently active
  paused, // Temporarily interrupted
  completed, // Finished
  archived, // Hidden but retrievable
  deleted, // Marked as deleted but not removed
}

void _assertStableEnumValuesStatus() {
  assert(0 == Status.unknown.index);
  assert(1 == Status.created.index);
  assert(2 == Status.scheduled.index);
  assert(3 == Status.inProgress.index);
  assert(4 == Status.paused.index);
  assert(5 == Status.completed.index);
  assert(6 == Status.archived.index);
  assert(7 == Status.deleted.index);
}

@Entity()
class Activity {
  @Id()
  int id = 0;

  String uuid;
  String name;

  @Property(type: PropertyType.date)
  DateTime? startDate;

  @Property(type: PropertyType.date)
  DateTime? endDate;

  @Transient()
  Kind kind;

  @Transient()
  Status status;

  String? tags; // JSON string for tags

  final task = ToOne<Task>();

  Activity({
    required this.uuid,
    required this.name,
    this.startDate,
    this.endDate,
    this.kind = Kind.activity,
    this.status = Status.created,
    this.tags,
  });

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! Activity) return false;
    return id == other.id && uuid == other.uuid;
  }

  int get dbKind {
    _assertStableEnumValuesKind();
    return kind.index;
  }

  set dbKind(int value) {
    kind =
        value > 0 && value < Kind.values.length
            ? Kind.values[value]
            : Kind.unknown;
  }

  int get dbStatus {
    _assertStableEnumValuesStatus();
    return status.index;
  }

  set dbStatus(int value) {
    status =
        value > 0 && value < Status.values.length
            ? Status.values[value]
            : Status.unknown;
  }
}
