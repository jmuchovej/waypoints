import 'package:tempo/src/database/objectbox/models/task.dart';
import 'package:tempo/src/database/sqlite/tables/schema.drift.dart';
import 'package:uuid/uuid.dart';

final ActivitySchemaV1 = Schema(
  uuid: const Uuid().v4(),
  name: 'activity',
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
      'description': 'The name of this activity',
    },
    'properties': {
      'kind': {
        'type': 'select',
        'enum': ActivityKind.values.map((s) => s.name).toList(),
        'default': ActivityKind.activity.name,
        'description': 'What kind of activity was this?',
      },
      'status': {
        'type': 'select',
        'enum': ActivityStatus.values.map((s) => s.name).toList(),
        'default': ActivityStatus.created.name,
        'description': 'Completion status of this activity',
      },
      'start_date': {
        'type': 'datetime',
        'description': 'When did this activity start?',
      },
      'end_date': {
        'type': 'datetime',
        'description': 'When did this activity stop?',
      },
      'task': {
        'type': 'reference',
        'reference': ['task'],
        'description': 'The task this activity belongs to',
      },
    },
  },
  display: {},
  indexed: {
    'kind': true,
    'status': true,
    'start_date': true,
    'end_date': true,
    'task': true,
  },
);

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
