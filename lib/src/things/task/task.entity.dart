import 'package:equatable/equatable.dart';
import 'package:flutter/material.dart';
import 'package:objectbox/objectbox.dart';
import 'package:super_editor/super_editor.dart';

import '../../editor/super_editor.dart';
import '../../objectbox/attributed_text_mixin.dart';
import '../../objectbox/objectbox.dart';
import '../../objectbox/objectbox.g.dart';
import '../foundation.dart';
import '../project/project.entity.dart';
import '../thing.entity.dart';

export 'task_urgency.dart';

// Resolver is now auto-registered by TaskRepository
// No need for manual resolver configuration!

enum TaskEnergy {
  /// Extra small - almost automatic
  XS(Colors.greenAccent),

  /// Small - Little energy
  S(Colors.green),

  /// Medium - Average energy
  M(Colors.amber),

  /// Large - Significant energy
  L(Colors.orange),

  /// Extra large - Peak energy required
  XL(Colors.red);

  const TaskEnergy(this.color);

  final Color color;

  static void assertStableEnumValues() {
    assert(0 == TaskEnergy.XS.index);
    assert(1 == TaskEnergy.S.index);
    assert(2 == TaskEnergy.M.index);
    assert(3 == TaskEnergy.L.index);
    assert(4 == TaskEnergy.XL.index);
  }

  @override
  String toString() => name.toUpperCase();
}

@Entity()
class TaskEntity
    extends ComparableThing
    with AttributedTextEntity, ThingBehavior
    implements Thing {
  TaskEntity({
    required this.uuid,
    AttributedText? title,
    DateTime? createDate,
    DateTime? modifyDate,
    DateTime? dueDate,
    this.startDate,
    this.endDate,
    this.scheduledDate,
    this.waitDate,
    this.untilDate,
    this.recurrence,
    this.archived = false,
    this.deleted = false,
    TaskStatus status = TaskStatus.todo,
    TaskTime time = TaskTime.M,
    TaskEnergy energy = TaskEnergy.M,
    TaskPriority priority = TaskPriority.M,
  }) : dueDate = dueDate ?? DateTime.now().add(Duration(days: 1)),
       createDate = createDate ?? DateTime.now(),
       modifyDate = modifyDate ?? DateTime.now(),
       _status = status,
       _time = time,
       _energy = energy,
       _priority = priority {
    this.title = title ?? AttributedText('');
  }

  static const key = 'task';

  @Id()
  @override
  int id = 0;

  @override
  String uuid;

  @Property(type: PropertyType.dateNano)
  @override
  DateTime createDate;

  @Property(type: PropertyType.dateNano)
  @override
  DateTime modifyDate;

  @Property(type: PropertyType.dateNano)
  DateTime? startDate;

  @Property(type: PropertyType.dateNano)
  DateTime? endDate;

  @Property(type: PropertyType.dateNano)
  DateTime? scheduledDate;

  @Property(type: PropertyType.dateNano)
  DateTime dueDate;

  @Property(type: PropertyType.dateNano)
  DateTime? waitDate;

  @Property(type: PropertyType.dateNano)
  DateTime? untilDate;

  String? recurrence;

  TaskStatus _status;
  TaskTime _time;
  TaskEnergy _energy;
  TaskPriority _priority;

  @override
  bool deleted;

  @override
  bool archived;

  /// Keep project relationship for now as it's a categorization
  final project = ToOne<ProjectEntity>();

  final history = ToMany<TaskSnapshotEntity>();

  @override
  final ToOne<ThingEntity> thingRef = ToOne<ThingEntity>();

  @override
  int get hashCode =>
      uuid.hashCode ^
      title.hashCode ^
      createDate.hashCode ^
      modifyDate.hashCode ^
      startDate.hashCode ^
      endDate.hashCode ^
      scheduledDate.hashCode ^
      dueDate.hashCode ^
      waitDate.hashCode ^
      untilDate.hashCode ^
      recurrence.hashCode ^
      statusOBX.hashCode ^
      timeOBX.hashCode ^
      energyOBX.hashCode ^
      priorityOBX.hashCode ^
      deleted.hashCode ^
      archived.hashCode;

  TaskEnergy get energy => _energy;
  set energyOBX(int value) => _energy = TaskEnergy.values[value];
  int get energyOBX {
    TaskEnergy.assertStableEnumValues();
    return _energy.index;
  }

  TaskPriority get priority => _priority;
  set priorityOBX(int value) => _priority = TaskPriority.values[value];
  int get priorityOBX {
    TaskPriority.assertStableEnumValues();
    return _priority.index;
  }

  TaskStatus get status => _status;
  set statusOBX(int value) => _status = TaskStatus.values[value];
  int get statusOBX {
    TaskStatus.assertStableEnumValues();
    return _status.index;
  }

  TaskTime get time => _time;
  set timeOBX(int value) => _time = TaskTime.values[value];
  int get timeOBX {
    TaskTime.assertStableEnumValues();
    return _time.index;
  }

  @override
  String get type => key;

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! TaskEntity) return false;
    return id == other.id && uuid == other.uuid;
  }

  TaskEntity copyWith({
    String? uuid,
    AttributedText? title,
    DateTime? createDate,
    DateTime? startDate,
    DateTime? endDate,
    DateTime? scheduledDate,
    DateTime? dueDate,
    DateTime? waitDate,
    DateTime? untilDate,
    String? recurrence,
    TaskStatus? status,
    TaskTime? time,
    TaskEnergy? energy,
    TaskPriority? priority,
    bool? deleted,
    bool? archived,
    ProjectEntity? project,
  }) {
    final task = TaskEntity(
      uuid: uuid ?? this.uuid,
      createDate: createDate ?? this.createDate,
      modifyDate: DateTime.now(),
      startDate: startDate ?? this.startDate,
      endDate: endDate ?? this.endDate,
      scheduledDate: scheduledDate ?? this.scheduledDate,
      dueDate: dueDate ?? this.dueDate,
      waitDate: waitDate ?? this.waitDate,
      untilDate: untilDate ?? this.untilDate,
      recurrence: recurrence ?? this.recurrence,
      status: status ?? this.status,
      time: time ?? this.time,
      energy: energy ?? this.energy,
      priority: priority ?? this.priority,
      deleted: deleted ?? this.deleted,
      archived: archived ?? this.archived,
    );
    task.id = id;
    task.title = title ?? this.title;
    task.thingRef.target = thingRef.target;

    return task;
  }

  @override
  String getSearchableText() => title.toPlainText();

  @override
  String toString() {
    return 'TaskEntity{id: $id, uuid: $uuid, title: "${title.toPlainText()}", startDate: $startDate, endDate: $endDate, scheduledDate: $scheduledDate, dueDate: $dueDate, waitDate: $waitDate, untilDate: $untilDate, recurrence: $recurrence, status: $status, time: $time, energy: $energy, priority: $priority, deleted: $deleted, archived: $archived}';
  }

  @override
  String titleJson = '';

  @Transient()
  @override
  AttributedText get title => getAttributedText(titleJson);
  @override
  set title(AttributedText value) =>
      setAttributedText(value, (json) => titleJson = json);

  @override
  Map<String, Object?> get propsMap => {
    "uuid": uuid,
    "title": title.toPlainText(),
    "startDate": startDate,
    "endDate": endDate,
    "scheduledDate": scheduledDate,
    "dueDate": dueDate,
    "waitDate": waitDate,
    "untilDate": untilDate,
    "recurrence": recurrence,
    "status": status,
    "time": time,
    "energy": energy,
    "priority": priority,
    "deleted": deleted,
    "archived": archived,
  };
}

@Entity()
class TaskSnapshotEntity implements ThingHistory {
  TaskSnapshotEntity({
    this.titleJson,
    this.startDate,
    this.endDate,
    this.scheduledDate,
    this.dueDate,
    this.waitDate,
    this.untilDate,
    this.recurrence,
    this.status,
    this.time,
    this.energy,
    this.priority,
    this.deleted,
    this.archived,
  });

  @Id()
  @override
  int id = 0;

  final task = ToOne<TaskEntity>();

  @override
  @Property(type: PropertyType.dateNano)
  DateTime timestamp = DateTime.now();

  /// Serialized AttributedText JSON
  String? titleJson;

  @Property(type: PropertyType.dateNano)
  DateTime? startDate;

  @Property(type: PropertyType.dateNano)
  DateTime? endDate;

  @Property(type: PropertyType.dateNano)
  DateTime? scheduledDate;

  @Property(type: PropertyType.dateNano)
  DateTime? dueDate;

  @Property(type: PropertyType.dateNano)
  DateTime? waitDate;

  @Property(type: PropertyType.dateNano)
  DateTime? untilDate;

  String? recurrence;
  int? status;
  int? time;
  int? energy;
  int? priority;

  @override
  bool? deleted;

  @override
  bool? archived;
}

enum TaskPriority {
  /// Low priority
  L('Low', Colors.green),

  /// Medium priority
  M('Medium', Colors.orange),

  /// High priority
  H('High', Colors.red);

  const TaskPriority(this.displayString, this.color);

  final String displayString;
  final Color color;

  static void assertStableEnumValues() {
    assert(0 == TaskPriority.L.index);
    assert(1 == TaskPriority.M.index);
    assert(2 == TaskPriority.H.index);
  }

  Widget buildIcon(BuildContext context) {
    return Icon(Icons.flag_circle_rounded, color: color);
  }

  @override
  String toString() => displayString;
}

enum TaskStatus {
  /// Initial state
  todo('To Do', Icons.radio_button_unchecked_rounded, Colors.white),

  /// We've started but not focused on it
  started('Started', Icons.pause_circle_rounded, Colors.blue),

  /// Blocked by something
  waiting('Waiting', Icons.pause_circle_rounded, Colors.orange),

  /// Interrupted but will resume
  paused('Paused', Icons.radio_button_unchecked_rounded, Colors.deepPurple),

  /// Parent task with child being worked on
  workInProgress(
    'Work in Progress',
    Icons.radio_button_unchecked_rounded,
    Colors.amber,
  ),

  /// Future task
  scheduled('Scheduled', Icons.radio_button_unchecked_rounded, Colors.teal),

  /// No longer working on it
  stopped('Stopped', Icons.radio_button_unchecked_rounded, Colors.redAccent),

  /// Decided not to complete
  dropped('Dropped', Icons.cancel_rounded, Colors.red),

  /// Finished! 🎉
  complete('Complete', Icons.check_circle_rounded, Colors.green);

  const TaskStatus(this.displayString, this.icon, this.color);

  final IconData icon;
  final String displayString;
  final Color color;

  static void assertStableEnumValues() {
    assert(0 == TaskStatus.todo.index);
    assert(1 == TaskStatus.started.index);
    assert(2 == TaskStatus.waiting.index);
    assert(3 == TaskStatus.paused.index);
    assert(4 == TaskStatus.workInProgress.index);
    assert(5 == TaskStatus.scheduled.index);
    assert(6 == TaskStatus.stopped.index);
    assert(7 == TaskStatus.dropped.index);
    assert(8 == TaskStatus.complete.index);
  }

  @override
  String toString() => displayString;

  Widget buildIcon(BuildContext context) {
    final Color color = Theme.of(context).primaryColor;
    return Icon(
      icon,
      color: color,
      applyTextScaling: true,
      semanticLabel: toString(),
    );
  }
}

enum TaskTime {
  /// Extra small - almost automatic
  XS(Colors.greenAccent),

  /// Small - quick task
  S(Colors.green),

  /// Medium - average task
  M(Colors.amber),

  /// Large - significant time
  L(Colors.orange),

  /// Extra large - major time commitment
  XL(Colors.red);

  const TaskTime(this.color);

  final Color color;

  static void assertStableEnumValues() {
    assert(0 == TaskTime.XS.index);
    assert(1 == TaskTime.S.index);
    assert(2 == TaskTime.M.index);
    assert(3 == TaskTime.L.index);
    assert(4 == TaskTime.XL.index);
  }

  @override
  String toString() => name.toUpperCase();
}

extension AllowedTransitions on TaskStatus {
  List<TaskStatus> get eligibleTransitions {
    switch (this) {
      case TaskStatus.todo:
        return [TaskStatus.started, TaskStatus.dropped];
      case TaskStatus.started:
        return [
          TaskStatus.waiting,
          TaskStatus.paused,
          TaskStatus.dropped,
          TaskStatus.complete,
        ];
      case TaskStatus.waiting:
        return [TaskStatus.paused, TaskStatus.stopped, TaskStatus.dropped];
      case TaskStatus.paused:
        return [
          TaskStatus.waiting,
          TaskStatus.stopped,
          TaskStatus.dropped,
          TaskStatus.complete,
        ];
      case TaskStatus.stopped:
        return [TaskStatus.dropped, TaskStatus.complete];
      case TaskStatus.scheduled:
      case TaskStatus.workInProgress:
      case TaskStatus.dropped:
      case TaskStatus.complete:
        return [];
    }
  }
}

class TaskStatusGroups {
  static List<TaskStatus> get awaiting => [
    TaskStatus.todo,
    TaskStatus.started,
    TaskStatus.waiting,
  ];

  static List<TaskStatus> get completing => [
    TaskStatus.complete,
    TaskStatus.dropped,
  ];

  static List<TaskStatus> get pending => [
    TaskStatus.paused,
    TaskStatus.workInProgress,
    TaskStatus.scheduled,
    TaskStatus.stopped,
  ];

  static Condition<TaskEntity> _convert(List<TaskStatus> statuses) {
    final indices = statuses.map((status) => status.index).toList();
    return TaskOBX.statusOBX.oneOf(indices);
  }

  static Condition<TaskEntity> get isAwaiting => _convert(awaiting);
  static Condition<TaskEntity> get isCompleted => _convert(completing);
  static Condition<TaskEntity> get isPending => _convert(pending);
}

extension StatusGroups on TaskStatus {
  bool get isAwaiting => TaskStatusGroups.awaiting.contains(this);
  bool get isCompleted => TaskStatusGroups.completing.contains(this);
  bool get isPending => TaskStatusGroups.pending.contains(this);
}
