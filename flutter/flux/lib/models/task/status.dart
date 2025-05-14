import 'package:flutter/material.dart';
import 'package:iconify/iconify.dart';
import 'package:iconify/icons/phosphor.dart';

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
}

void assertStableEnumValues() {
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
}

extension AllowedTransitions on Status {
  List<Status> get eligibleTransitions {
    switch (this) {
      case Status.todo:
        return [Status.started, Status.dropped];
      case Status.started:
        return [Status.waiting, Status.paused, Status.dropped, Status.complete];
      case Status.waiting:
        return [Status.paused, Status.stopped, Status.dropped];
      case Status.paused:
        return [
          Status.waiting,
          Status.stopped,
          Status.dropped,
          Status.complete,
        ];
      case Status.stopped:
        return [Status.dropped, Status.complete];
      case Status.unknown:
      case Status.scheduled:
      case Status.workInProgress:
      case Status.dropped:
      case Status.complete:
        return [];
    }
  }
}

extension StatusGroups on Status {
  bool get isCompleted {
    return [Status.dropped, Status.complete].contains(this);
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

extension StatusColors on Status {
  Color get color {
    switch (this) {
      case Status.todo:
        return Colors.grey;
      case Status.started:
        return Colors.blue;
      case Status.waiting:
        return Colors.orange;
      case Status.paused:
        return Colors.deepPurple;
      case Status.workInProgress:
        return Colors.amber;
      case Status.scheduled:
        return Colors.teal;
      case Status.stopped:
        return Colors.red;
      case Status.dropped:
        return Colors.red[900]!;
      case Status.complete:
        return Colors.green;
      // case Status.archived:
      //   color = Colors.grey[700]!;
      // case Status.deleted:
      //   color = Colors.red[900]!;
      default:
        return Colors.grey;
    }
  }
}

extension StatusIcons on Status {
  Widget get icon {
    String icon;

    switch (this) {
      case Status.todo:
        icon = Phosphor["circle-bold"];
        break;
      case Status.started:
        icon = Phosphor["play-circle-bold"];
        break;
      case Status.waiting:
        icon = Phosphor["hourglass-bold"];
        break;
      case Status.paused:
        icon = Phosphor["pause-circle-bold"];
        break;
      case Status.workInProgress:
        icon = Phosphor["traffic-cone-bold"];
        break;
      case Status.scheduled:
        icon = Phosphor["calendar-dot-bold"];
        break;
      case Status.stopped:
        icon = Phosphor["stop-circle-bold"];
        break;
      case Status.dropped:
        icon = Phosphor["x-circle-bold"];
        break;
      case Status.complete:
        icon = Phosphor["check-circle-bold"];
        break;
      // case Status.archived:
      //   icon = Phosphor["archive-box-fill"];
      //   break;
      // case Status.deleted:
      //   icon = Phosphor["trash-bold"];
      //   break;
      case Status.unknown:
        icon = Phosphor["seal-question-fill"];
        break;
    }

    return Iconify(icon, color: color);
  }
}

extension StatusDisplayString on Status {
  String toDisplayString() {
    switch (this) {
      case Status.unknown:
        return 'Unknown';
      case Status.todo:
        return 'To Do';
      case Status.started:
        return 'Started';
      case Status.waiting:
        return 'Waiting';
      case Status.paused:
        return 'Paused';
      case Status.workInProgress:
        return 'Work in Progress';
      case Status.scheduled:
        return 'Scheduled';
      case Status.stopped:
        return 'Stopped';
      case Status.dropped:
        return 'Dropped';
      case Status.complete:
        return 'Complete';
    }
  }
}
