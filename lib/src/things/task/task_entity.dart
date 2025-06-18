import 'package:flutter/material.dart';
import 'package:objectbox/objectbox.dart';

import '../../editor/super_editor.dart';
import '../../objectbox/attributed_text_mixin.dart';
import '../project/project_entity.dart';
import '../thing/interface.dart';
import '../thing/thing_entity.dart';

export 'task_urgency.dart';

// Resolver is now auto-registered by TaskRepository
// No need for manual resolver configuration!

enum TaskEnergyEstimate {
  XS, // Almost no energy
  S, // Little energy
  M, // Medium energy
  L, // Significant energy
  XL; // Peak energy required

  static void assertStableEnumValues() {
    assert(0 == TaskEnergyEstimate.XS.index);
    assert(1 == TaskEnergyEstimate.S.index);
    assert(2 == TaskEnergyEstimate.M.index);
    assert(3 == TaskEnergyEstimate.L.index);
    assert(4 == TaskEnergyEstimate.XL.index);
  }
}

@Entity()
class TaskEntity with AttributedTextEntity, ThingBehavior implements Thing {
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
    TaskTimeEstimate timeEstimate = TaskTimeEstimate.M,
    TaskEnergyEstimate energyEstimate = TaskEnergyEstimate.M,
    TaskPriority priority = TaskPriority.M,
  }) : dueDate = dueDate ?? DateTime.now().add(Duration(days: 1)),
       createDate = createDate ?? DateTime.now(),
       modifyDate = modifyDate ?? DateTime.now(),
       _status = status,
       _timeEstimate = timeEstimate,
       _energyEstimate = energyEstimate,
       _priority = priority {
    this.title = title ?? AttributedText('');
  }

  static const key = 'task';

  @Id()
  @override
  int id = 0;

  @override
  String uuid;

  /// Serialized AttributedText JSON
  String titleJson = '';

  /// Plain text version of title for querying
  @Index()
  String titlePlainText = '';

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

  TaskTimeEstimate _timeEstimate;
  TaskEnergyEstimate _energyEstimate;
  TaskPriority _priority;
  @override
  bool deleted;

  @override
  bool archived;

  /// Polymorphic children relationship - can contain any entity type
  /// When viewing a task, these are the document nodes shown in order
  @override
  final children = ToMany<ThingEntity>();

  /// Direct reference to the ThingEntity registration
  /// This creates a 1:1 relationship between this Task and its registry entry
  @override
  final ToOne<ThingEntity> thing = ToOne<ThingEntity>();

  // Keep project relationship for now as it's a categorization
  final project = ToOne<ProjectEntity>();

  // Keep parent for task hierarchy tracking, but relationships
  // will be managed through polymorphic children
  final parent = ToOne<TaskEntity>();

  final history = ToMany<TaskSnapshotEntity>();

  @Transient()
  TaskEnergyEstimate get energyEstimate => _energyEstimate;

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
      status.hashCode ^
      timeEstimate.hashCode ^
      energyEstimate.hashCode ^
      priority.hashCode ^
      deleted.hashCode ^
      archived.hashCode;

  int get obxEnergyEstimate {
    TaskEnergyEstimate.assertStableEnumValues();
    return _energyEstimate.index;
  }

  set obxEnergyEstimate(int value) =>
      _energyEstimate = TaskEnergyEstimate.values[value];

  int get obxPriority {
    TaskPriority.assertStableEnumValues();
    return _priority.index;
  }

  set obxPriority(int value) => _priority = TaskPriority.values[value];
  // String get obxNotes => jsonEncode(_notes.toJson());
  // set obxNotes(String value) => _notes = Delta.fromJson(jsonDecode(value));
  // @Transient()
  // Delta get notes => _notes;

  int get obxStatus {
    TaskStatus.assertStableEnumValues();
    return _status.index;
  }

  set obxStatus(int value) => _status = TaskStatus.values[value];

  int get obxTimeEstimate {
    TaskTimeEstimate.assertStableEnumValues();
    return _timeEstimate.index;
  }

  set obxTimeEstimate(int value) =>
      _timeEstimate = TaskTimeEstimate.values[value];

  @Transient()
  TaskPriority get priority => _priority;

  @Transient()
  TaskStatus get status => _status;
  @Transient()
  TaskTimeEstimate get timeEstimate => _timeEstimate;

  // String get obxTitle => jsonEncode(_title.toJson());
  // set obxTitle(String value) => _title = Delta.fromJson(jsonDecode(value));
  @Transient()
  AttributedText get title => getAttributedText(titleJson, titlePlainText);

  set title(AttributedText value) => setAttributedText(
    value,
    (json) => titleJson = json,
    (plain) => titlePlainText = plain,
  );

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
    TaskTimeEstimate? timeEstimate,
    TaskEnergyEstimate? energyEstimate,
    TaskPriority? priority,
    bool? deleted,
    bool? archived,
    TaskEntity? parent,
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
      timeEstimate: timeEstimate ?? this.timeEstimate,
      energyEstimate: energyEstimate ?? this.energyEstimate,
      priority: priority ?? this.priority,
      deleted: deleted ?? this.deleted,
      archived: archived ?? this.archived,
    );
    task.id = id;
    task.title = title ?? this.title;

    task.parent.target = parent ?? this.parent.target;
    // Note: children relationships should be copied separately via polymorphic repository
    task.project.target = project ?? this.project.target;

    return task;
  }

  @override
  String toString() {
    return 'TaskEntity{id: $id, uuid: $uuid, title: $title, startDate: $startDate, endDate: $endDate, scheduledDate: $scheduledDate, dueDate: $dueDate, waitDate: $waitDate, untilDate: $untilDate, recurrence: $recurrence, status: $status, timeEstimate: $timeEstimate, energyEstimate: $energyEstimate, priority: $priority, deleted: $deleted, archived: $archived}';
  }
}

@Entity()
class TaskSnapshotEntity {
  TaskSnapshotEntity({
    this.titleJson,
    this.titlePlainText,
    this.startDate,
    this.endDate,
    this.scheduledDate,
    this.dueDate,
    this.waitDate,
    this.untilDate,
    this.recurrence,
    this.status,
    this.timeEstimate,
    this.energyEstimate,
    this.priority,
    this.deleted,
    this.archived,
  });
  @Id()
  int id = 0;

  final task = ToOne<TaskEntity>();

  @Property(type: PropertyType.dateNano)
  DateTime timestamp = DateTime.now();

  /// Serialized AttributedText JSON
  String? titleJson;

  /// Plain text version of title for querying
  @Index()
  String? titlePlainText;

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
  int? timeEstimate;
  int? energyEstimate;
  int? priority;
  bool? deleted;
  bool? archived;
}

enum TaskPriority {
  L, // Low priority
  M, // Medium priority
  H; // High priority

  static void assertStableEnumValues() {
    assert(0 == TaskPriority.L.index);
    assert(1 == TaskPriority.M.index);
    assert(2 == TaskPriority.H.index);
  }
}

enum TaskStatus {
  todo, // Initial state
  started, // We've started but not focused on it
  waiting, // Blocked by something
  paused, // Interrupted but will resume
  workInProgress, // Parent task with child being worked on
  scheduled, // Future task
  stopped, // No longer working on it
  dropped, // Decided not to complete
  complete; // Finished! 🎉

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
}

enum TaskTimeEstimate {
  XS, // Extra small - almost automatic
  S, // Small - quick task
  M, // Medium - average task
  L, // Large - significant time
  XL; // Extra large - major time commitment

  static void assertStableEnumValues() {
    assert(0 == TaskTimeEstimate.XS.index);
    assert(1 == TaskTimeEstimate.S.index);
    assert(2 == TaskTimeEstimate.M.index);
    assert(3 == TaskTimeEstimate.L.index);
    assert(4 == TaskTimeEstimate.XL.index);
  }
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

extension EstimatedEnergyColors on TaskEnergyEstimate {
  Color get color {
    switch (this) {
      case TaskEnergyEstimate.XS:
        return Colors.green.shade300;
      case TaskEnergyEstimate.S:
        return Colors.green;
      case TaskEnergyEstimate.M:
        return Colors.amber;
      case TaskEnergyEstimate.L:
        return Colors.orange;
      case TaskEnergyEstimate.XL:
        return Colors.red;
    }
  }
}

extension EstimatedEnergyDisplayString on TaskEnergyEstimate {
  String toDisplayString() => name.toUpperCase();
}

extension EstimatedTimeColors on TaskTimeEstimate {
  Color get color {
    switch (this) {
      case TaskTimeEstimate.XS:
        return Colors.green.shade300;
      case TaskTimeEstimate.S:
        return Colors.green;
      case TaskTimeEstimate.M:
        return Colors.amber;
      case TaskTimeEstimate.L:
        return Colors.orange;
      case TaskTimeEstimate.XL:
        return Colors.red;
    }
  }
}

extension EstimatedTimeDisplayString on TaskTimeEstimate {
  String toDisplayString() => name.toUpperCase();
}

extension PriorityColors on TaskPriority {
  Color get color {
    switch (this) {
      case TaskPriority.L:
        return Colors.green;
      case TaskPriority.M:
        return Colors.orange;
      case TaskPriority.H:
        return Colors.red;
    }
  }
}

extension PriorityDisplayString on TaskPriority {
  String toDisplayString() {
    switch (this) {
      case TaskPriority.L:
        return 'Low';
      case TaskPriority.M:
        return 'Medium';
      case TaskPriority.H:
        return 'High';
    }
  }
}

extension PriorityIcons on TaskPriority {
  Widget get icon {
    return Icon(Icons.flag_circle_rounded, color: color);
  }
}

extension StatusColors on TaskStatus {
  Color get color {
    switch (this) {
      case TaskStatus.todo:
        return Colors.grey;
      case TaskStatus.started:
        return Colors.blue;
      case TaskStatus.waiting:
        return Colors.orange;
      case TaskStatus.paused:
        return Colors.deepPurple;
      case TaskStatus.workInProgress:
        return Colors.amber;
      case TaskStatus.scheduled:
        return Colors.teal;
      case TaskStatus.stopped:
        return Colors.red;
      case TaskStatus.dropped:
        return Colors.red[900]!;
      case TaskStatus.complete:
        return Colors.green;
      // case Status.archived:
      //   color = Colors.grey[700]!;
      // case Status.deleted:
      //   color = Colors.red[900]!;
    }
  }
}

extension StatusDisplayString on TaskStatus {
  String toDisplayString() {
    switch (this) {
      case TaskStatus.todo:
        return 'To Do';
      case TaskStatus.started:
        return 'Started';
      case TaskStatus.waiting:
        return 'Waiting';
      case TaskStatus.paused:
        return 'Paused';
      case TaskStatus.workInProgress:
        return 'Work in Progress';
      case TaskStatus.scheduled:
        return 'Scheduled';
      case TaskStatus.stopped:
        return 'Stopped';
      case TaskStatus.dropped:
        return 'Dropped';
      case TaskStatus.complete:
        return 'Complete';
    }
  }
}

extension StatusGroups on TaskStatus {
  bool get isAwaiting {
    return [
      TaskStatus.todo,
      TaskStatus.started,
      TaskStatus.waiting,
    ].contains(this);
  }

  bool get isCompleted {
    return [TaskStatus.dropped, TaskStatus.complete].contains(this);
  }

  bool get isPending {
    return [
      TaskStatus.paused,
      TaskStatus.workInProgress,
      TaskStatus.scheduled,
      TaskStatus.stopped,
    ].contains(this);
  }
}

extension StatusIcons on TaskStatus {
  IconData get icon {
    if (this == TaskStatus.complete) return Icons.check_circle_rounded;
    if (this == TaskStatus.dropped) return Icons.cancel_rounded;
    if (isAwaiting) return Icons.pause_circle_rounded;

    return Icons.radio_button_unchecked_rounded;
  }

  Widget buildIcon(BuildContext context) {
    final Color color = Theme.of(context).primaryColor;
    return Icon(
      icon,
      color: color,
      applyTextScaling: true,
      semanticLabel: toDisplayString(),
    );
  }
}
