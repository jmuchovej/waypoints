import 'package:flutter/material.dart';
import 'package:tempo/src/database/sqlite/tables/schema.drift.dart';
import 'package:uuid/uuid.dart';

final TaskSchemaV1 = Schema(
  uuid: Uuid().v4(),
  name: 'task',
  version: 1,
  deleted: false,
  archived: false,
  createdAt: DateTime(2025),
  updatedAt: DateTime.now(),
  systemDefined: true,
  definition: {
    'type': 'object',
    'title': {
      'type': 'string',
      'description': 'The name of this task',
    },
    'properties': {
      'status': {
        'type': 'select',
        'enum': TaskStatus.values.map((s) => s.name).toList(),
        'default': TaskStatus.todo.name,
        'description': 'Completion status of this task',
      },
      'priority': {
        'type': 'select',
        'enum': TaskPriority.values.map((s) => s.name).toList(),
        'default': TaskPriority.M.name,
        'description': 'Relative priority of this task',
      },
      'time_estimate': {
        'type': 'select',
        'enum': TaskTimeEstimate.values.map((s) => s.name).toList(),
        'default': TaskTimeEstimate.M.name,
        'description': 'Relative time estimate of this task',
      },
      'energy_estimate': {
        'type': 'select',
        'enum': TaskEnergyEstimate.values.map((s) => s.name).toList(),
        'default': TaskEnergyEstimate.M.name,
        'description': 'Relative energy estimate of this task',
      },
      'due_date': {
        'type': 'datetime',
        'description': 'Hard-stop date that this task must be completed by',
      },
      'start_date': {
        'type': 'datetime',
        'description': 'The date you started working on this date',
      },
      'end_date': {
        'type': 'datetime',
        'description': 'The date this task entered a "completed" state',
      },
      'scheduled_date': {
        'type': 'datetime',
        'description': 'When you want to start working on this task',
      },
      'wait_date': {
        'type': 'datetime',
        'description': 'The earliest date you want to consider this task "actionable"',
      },
      'until_date': {
        'type': 'datetime',
        'description': 'For recurring tasks, when do you want recurrence to stop?',
      },
      'recurrence': {
        'type': 'string',
        'description': 'Recurrence pattern for a repeating task',
      },
      'project': {
        'type': 'reference',
        'reference': ['project'],
        'description': 'Project that this task belongs to',
      },
      'parent': {
        'type': 'reference',
        'reference': ['task'],
        'description': 'Task that is blocked by the completion status of this task',
      },
      'children': {
        'type': 'array',
        'items': {
          'type': 'reference',
          'reference': ['task'],
        },
        'description': 'Tasks that block the completion of this task',
      },
      'activities': {
        'type': 'array',
        'items': {
          'type': 'reference',
          'reference': ['activity'],
        },
        'description': 'Activities that contributed towards this task\'s completion.'
      }
    },
  },
  display: {},
  indexed: {
    'status': true,
    'priority': true,
    'due_date': true,
    'project': true,
    'parent': true,
  },
);

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
  String toDisplayString() {
    switch (this) {
      case TaskEnergyEstimate.XS:
        return 'XS';
      case TaskEnergyEstimate.S:
        return 'S';
      case TaskEnergyEstimate.M:
        return 'M';
      case TaskEnergyEstimate.L:
        return 'L';
      case TaskEnergyEstimate.XL:
        return 'XL';
    }
  }
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

extension PriortyDisplayString on TaskPriority {
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
    return Icon(icon, color: color, applyTextScaling: true, semanticLabel: toDisplayString());
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
  String toDisplayString() {
    switch (this) {
      case TaskTimeEstimate.XS:
        return 'XS';
      case TaskTimeEstimate.S:
        return 'S';
      case TaskTimeEstimate.M:
        return 'M';
      case TaskTimeEstimate.L:
        return 'L';
      case TaskTimeEstimate.XL:
        return 'XL';
    }
  }
}
