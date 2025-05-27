import 'package:tempo/src/database/objectbox/models/task.dart';
import 'package:tempo/src/database/sqlite/tables/schema.drift.dart';
import 'package:uuid/uuid.dart';

final ProjectSchemaV1 = Schema(
  uuid: Uuid().v4(),
  name: 'project',
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
      'description': 'The name of this project',
    },
    'properties': {
      'status': {
        'type': 'select',
        'enum': ProjectStatus.values.map((s) => s.name).toList(),
        'default': ProjectStatus.planning.name,
        'description': 'Completion status of this project',
      },
      'priority': {
        'type': 'select',
        'enum': ProjectPriority.values.map((s) => s.name).toList(),
        'default': ProjectPriority.M.name,
        'description': 'Relative priority of this project',
      },
      'start_date': {
        'type': 'datetime',
        'description': 'When the project started',
      },
      'end_date': {
        'type': 'datetime',
        'description': 'When the project finished',
      },
      'due_date': {
        'type': 'datetime',
        'description': 'When you intend to complete this project by',
      },
      'parent': {
        'type': 'reference',
        'reference': ['project'],
        'description': 'A project this project belongs to',
      },
      'children': {
        'type': 'array',
        'items': {
          'type': 'reference',
          'reference': ['project'],
        },
        'description': 'Projects that belong to this project',
      },
      'tasks': {
        'type': 'array',
        'items': {
          'type': 'reference',
          'reference': ['task'],
        },
        'description': 'The tasks that belong to this project',
      },
    },
  },
  display: {},
  indexed: {
    'status': true,
    'start_date': true,
    'end_date': true,
    'due_date': true,
    'parent': true,
  },
);

enum ProjectStatus {
  planning, // Initial state
  active, // We've started but not focused on it
  paused, // Interrupted but will resume
  dropped, // Decided not to complete
  complete; // Finished! 🎉

  static void assertStableEnumValues() {
    assert(0 == ProjectStatus.planning.index);
    assert(1 == ProjectStatus.active.index);
    assert(2 == ProjectStatus.paused.index);
    assert(3 == ProjectStatus.dropped.index);
    assert(4 == ProjectStatus.complete.index);
  }
}

enum ProjectPriority {
  L, // Low priority
  M, // Medium priority
  H; // High priority

  static void assertStableEnumValues() {
    assert(0 == ProjectPriority.L.index);
    assert(1 == ProjectPriority.M.index);
    assert(2 == ProjectPriority.H.index);
  }
}
