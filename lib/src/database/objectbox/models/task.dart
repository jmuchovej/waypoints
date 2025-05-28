import 'package:flutter/material.dart';
import 'package:objectbox/objectbox.dart';
import 'package:tempo/src/database/objectbox/models/project.dart';
import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/database/objectbox/converters/attributed_text.dart';
import 'package:tempo/src/editor/attributed_text.dart';
import 'dart:convert';

import 'package:tempo/src/infrastructure.dart';

@Entity()
class TaskEntity with AttributedTextEntity {
  @Id()
  int id = 0;

  String uuid;

  /// Serialized AttributedText JSON
  String titleJson = '';

  /// Plain text version of title for querying
  @Index()
  String titlePlainText = '';

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

  TaskStatus _status;
  TaskTimeEstimate _timeEstimate;
  TaskEnergyEstimate _energyEstimate;
  TaskPriority _priority;

  bool deleted;
  bool archived;

  final parent = ToOne<TaskEntity>();
  @Backlink()
  final children = ToMany<TaskEntity>();

  final project = ToOne<ProjectEntity>();

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
    List<TaskEntity>? children,
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
    task.title = title ?? this.title;

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

  // String get obxTitle => jsonEncode(_title.toJson());
  // set obxTitle(String value) => _title = Delta.fromJson(jsonDecode(value));
  @Transient()
  AttributedText get title => getAttributedText(titleJson, titlePlainText);
  set title(AttributedText value) => setAttributedText(
    value,
    (json) => titleJson = json,
    (plain) => titlePlainText = plain,
  );

  // String get obxNotes => jsonEncode(_notes.toJson());
  // set obxNotes(String value) => _notes = Delta.fromJson(jsonDecode(value));
  // @Transient()
  // Delta get notes => _notes;

  int get obxStatus {
    TaskStatus.assertStableEnumValues();
    return _status.index;
  }

  set obxStatus(int value) => _status = TaskStatus.values[value];
  @Transient()
  TaskStatus get status => _status;

  int get obxEnergyEstimate {
    TaskEnergyEstimate.assertStableEnumValues();
    return _energyEstimate.index;
  }

  set obxEnergyEstimate(int value) =>
      _energyEstimate = TaskEnergyEstimate.values[value];
  @Transient()
  TaskEnergyEstimate get energyEstimate => _energyEstimate;

  int get obxTimeEstimate {
    TaskTimeEstimate.assertStableEnumValues();
    return _timeEstimate.index;
  }

  set obxTimeEstimate(int value) =>
      _timeEstimate = TaskTimeEstimate.values[value];
  @Transient()
  TaskTimeEstimate get timeEstimate => _timeEstimate;

  int get obxPriority {
    TaskPriority.assertStableEnumValues();
    return _priority.index;
  }

  set obxPriority(int value) => _priority = TaskPriority.values[value];
  @Transient()
  TaskPriority get priority => _priority;
}

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

extension PriorityIcons on TaskPriority {
  Widget get icon {
    return Icon(Icons.flag_circle_rounded, color: color);
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

extension StatusGroups on TaskStatus {
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

  bool get isAwaiting {
    return [
      TaskStatus.todo,
      TaskStatus.started,
      TaskStatus.waiting,
    ].contains(this);
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

extension StatusDisplayString on TaskStatus {
  @override
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
