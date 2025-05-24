import 'package:objectbox/objectbox.dart';
import 'package:tempo/core/models/task.dart';

@Entity()
class ActivityEntity {
  @Id()
  int id = 0;

  String uuid;
  String title;

  @Property(type: PropertyType.date)
  DateTime? startDate;

  @Property(type: PropertyType.date)
  DateTime? endDate;

  @Transient()
  ActivityKind kind;

  @Transient()
  ActivityStatus status;

  List<String>? tags;

  final task = ToOne<TaskEntity>();

  bool archived;
  bool deleted;

  ActivityEntity({
    required this.uuid,
    required this.title,
    this.startDate,
    this.endDate,
    this.kind = ActivityKind.activity,
    this.status = ActivityStatus.created,
    this.tags,
    this.archived = false,
    this.deleted = false,
  });

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! ActivityEntity) return false;
    return id == other.id && uuid == other.uuid;
  }

  @override
  int get hashCode =>
      uuid.hashCode ^
      title.hashCode ^
      startDate.hashCode ^
      endDate.hashCode ^
      kind.hashCode ^
      status.hashCode ^
      tags.hashCode ^
      archived.hashCode ^
      deleted.hashCode;

  int get dbKind {
    ActivityKind.assertStableEnumValues();
    return kind.index;
  }

  set dbKind(int value) {
    assert(0 < value && value < ActivityKind.values.length);
    kind = ActivityKind.values[value];
  }

  int get dbStatus {
    ActivityStatus.assertStableEnumValues();
    return status.index;
  }

  set dbStatus(int value) {
    assert(0 < value && value < ActivityStatus.values.length);
    status = ActivityStatus.values[value];
  }
}

enum ActivityStatus {
  created, // Initial state
  scheduled, // Start time assigned but not started
  active, // Currently active
  paused, // Temporarily interrupted
  completed; // Finished

  static void assertStableEnumValues() {
    assert(0 == ActivityStatus.created.index);
    assert(1 == ActivityStatus.scheduled.index);
    assert(2 == ActivityStatus.active.index);
    assert(3 == ActivityStatus.paused.index);
    assert(4 == ActivityStatus.completed.index);
  }
}

enum ActivityKind {
  activity, // Generic activity
  task, // Working on task
  event, // Calendar event
  intervalWork, // Work period (Pomodoro)
  intervalRest, // Rest period (Pomodoro)
  timeBlock; // Time-block with related tasks

  static void assertStableEnumValues() {
    assert(0 == ActivityKind.activity.index);
    assert(1 == ActivityKind.task.index);
    assert(2 == ActivityKind.event.index);
    assert(3 == ActivityKind.intervalWork.index);
    assert(4 == ActivityKind.intervalRest.index);
    assert(5 == ActivityKind.timeBlock.index);
  }
}
