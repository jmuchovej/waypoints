import 'package:flutter/material.dart';

enum TaskStatus {
  todo, // Initial state
  started, // We've started but not focused on it
  waiting, // Blocked by something
  paused, // Interrupted but will resume
  workInProgress, // Parent task with child being worked on
  scheduled, // Future task
  stopped, // No longer working on it
  dropped, // Decided not to complete
  complete, // Finished! 🎉
}

void assertStableEnumValues() {
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
  Widget get icon {
    IconData icon;

    switch (this) {
      case TaskStatus.todo:
        icon = Icons.circle_rounded;
        break;
      case TaskStatus.started:
        icon = Icons.play_circle_rounded;
        break;
      case TaskStatus.waiting:
        icon = Icons.hourglass_top_rounded;
        break;
      case TaskStatus.paused:
        icon = Icons.pause_circle_rounded;
        break;
      case TaskStatus.workInProgress:
        icon = Icons.construction_rounded;
        break;
      case TaskStatus.scheduled:
        icon = Icons.calendar_month_rounded;
        break;
      case TaskStatus.stopped:
        icon = Icons.stop_circle_rounded;
        break;
      case TaskStatus.dropped:
        icon = Icons.dangerous_rounded;
        break;
      case TaskStatus.complete:
        icon = Icons.check_circle_rounded;
        break;
      // case Status.archived:
      //   icon = Phosphor.archiveboxfill;
      //   break;
      // case Status.deleted:
      //   icon = Phosphor.trashbold;
      //   break;
    }

    return Icon(icon, color: color);
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
