// lib/services/seed_data_service.dart
import 'package:flutter/foundation.dart';
import 'package:uuid/uuid.dart';
import 'package:flux/models/task.dart';
import 'package:flux/models/task_list.dart';
import 'package:flux/models/activity.dart';
import 'package:flux/objectbox.dart';
import 'package:flux/utils/logging.dart';

class SeedDataService {
  static bool _hasSeeded = false;

  /// Seeds the database with sample data if in development mode
  /// and if the database is empty
  static Future<void> seedIfNeeded({bool force = false}) async {
    // Skip if already seeded and not forced
    if (_hasSeeded && !force) return;

    // Check if in development mode
    if (!kDebugMode && !force) return;

    // Check if database is empty
    final taskCount = ObjectBox.taskBox().count();
    final projectCount = ObjectBox.taskListBox().count();

    if ((taskCount == 0 && projectCount == 0) || force) {
      await _seedSampleData();
      _hasSeeded = true;
      logger.i('Sample data seeded successfully');
    } else {
      logger.d('Database already contains data, skipping seed');
    }
  }

  /// Seeds the database with sample projects and tasks
  static Future<void> _seedSampleData() async {
    // Create sample projects
    final projects = _createSampleProjects();
    final projectIds = ObjectBox.taskListBox().putMany(projects);
    logger.d('Created ${projectIds.length} sample projects');

    // Reload projects to get their assigned IDs
    final savedProjects = ObjectBox.taskListBox().getAll();

    // Create sample tasks for each project
    final allTasks = <Task>[];

    for (final project in savedProjects) {
      final tasks = _createSampleTasksForProject(project);
      allTasks.addAll(tasks);
    }

    // Save all tasks
    final taskIds = ObjectBox.taskBox().putMany(allTasks);
    logger.d('Created ${taskIds.length} sample tasks');

    // Create sample activities
    final activities = _createSampleActivities(allTasks);
    final activityIds = ObjectBox.activityBox().putMany(activities);
    logger.d('Created ${activityIds.length} sample activities');
  }

  /// Creates sample projects
  static List<TaskList> _createSampleProjects() {
    return [
      TaskList(
        uuid: const Uuid().v4(),
        name: 'Personal',
        description: 'Personal tasks and projects',
      ),
      TaskList(
        uuid: const Uuid().v4(),
        name: 'Work',
        description: 'Work-related tasks and projects',
      ),
      TaskList(
        uuid: const Uuid().v4(),
        name: 'Health',
        description: 'Health and wellness tasks',
      ),
      TaskList(
        uuid: const Uuid().v4(),
        name: 'Research',
        description: 'Research projects and literature review',
      ),
    ];
  }

  /// Creates sample tasks for a given project
  static List<Task> _createSampleTasksForProject(TaskList project) {
    final now = DateTime.now();
    final tasks = <Task>[];

    // Create different tasks based on project type
    switch (project.name) {
      case 'Personal':
        tasks.addAll([
          _createTask(
            'Buy groceries',
            'Milk, eggs, bread, vegetables',
            project,
            dueDate: now.add(const Duration(days: 1)),
            status: TaskStatus.todo,
            priority: TaskPriority.M,
            estimatedTime: TaskTimeEstimate.S,
            estimatedEnergy: TaskEnergyEstimate.M,
          ),
          _createTask(
            'Clean apartment',
            'Vacuum, dust, clean bathroom',
            project,
            dueDate: now.add(const Duration(days: 3)),
            status: TaskStatus.scheduled,
            priority: TaskPriority.L,
            estimatedTime: TaskTimeEstimate.M,
            estimatedEnergy: TaskEnergyEstimate.L,
          ),
          _createTask(
            'Plan weekend trip',
            'Research destinations, book accommodations',
            project,
            dueDate: now.add(const Duration(days: 5)),
            status: TaskStatus.started,
            priority: TaskPriority.H,
            estimatedTime: TaskTimeEstimate.M,
            estimatedEnergy: TaskEnergyEstimate.M,
          ),
        ]);
        break;

      case 'Work':
        tasks.addAll([
          _createTask(
            'Complete quarterly report',
            'Analyze data and prepare presentation',
            project,
            dueDate: now.add(const Duration(days: 7)),
            status: TaskStatus.workInProgress,
            priority: TaskPriority.H,
            estimatedTime: TaskTimeEstimate.L,
            estimatedEnergy: TaskEnergyEstimate.L,
          ),
          _createTask(
            'Review team members code',
            'Code review for the new feature implementation',
            project,
            dueDate: now.add(const Duration(days: 2)),
            status: TaskStatus.waiting,
            priority: TaskPriority.M,
            estimatedTime: TaskTimeEstimate.M,
            estimatedEnergy: TaskEnergyEstimate.M,
          ),
          _createTask(
            'Prepare for client meeting',
            'Create agenda and gather materials',
            project,
            dueDate: now.add(const Duration(days: 1)),
            status: TaskStatus.todo,
            priority: TaskPriority.H,
            estimatedTime: TaskTimeEstimate.S,
            estimatedEnergy: TaskEnergyEstimate.M,
          ),
          _createTask(
            'Update documentation',
            'Add new API endpoints to the documentation',
            project,
            dueDate: now.subtract(const Duration(days: 2)),
            status: TaskStatus.complete,
            priority: TaskPriority.L,
            estimatedTime: TaskTimeEstimate.M,
            estimatedEnergy: TaskEnergyEstimate.S,
          ),
        ]);
        break;

      case 'Health':
        tasks.addAll([
          _createTask(
            'Morning workout',
            '30 minutes cardio, 15 minutes strength training',
            project,
            dueDate: now,
            status: TaskStatus.paused,
            priority: TaskPriority.M,
            estimatedTime: TaskTimeEstimate.S,
            estimatedEnergy: TaskEnergyEstimate.M,
          ),
          _createTask(
            'Schedule annual checkup',
            'Call doctor\'s office for appointment',
            project,
            dueDate: now.add(const Duration(days: 14)),
            status: TaskStatus.todo,
            priority: TaskPriority.M,
            estimatedTime: TaskTimeEstimate.XS,
            estimatedEnergy: TaskEnergyEstimate.XS,
          ),
          _createTask(
            'Meal prep for the week',
            'Plan meals and prepare ingredients',
            project,
            dueDate: now.add(const Duration(days: 2)),
            status: TaskStatus.scheduled,
            priority: TaskPriority.M,
            estimatedTime: TaskTimeEstimate.M,
            estimatedEnergy: TaskEnergyEstimate.M,
          ),
        ]);
        break;

      case 'Research':
        tasks.addAll([
          _createTask(
            'Literature review',
            'Read recent papers on computational cognitive science',
            project,
            dueDate: now.add(const Duration(days: 5)),
            status: TaskStatus.started,
            priority: TaskPriority.H,
            estimatedTime: TaskTimeEstimate.L,
            estimatedEnergy: TaskEnergyEstimate.L,
          ),
          _createTask(
            'Revise experiment design',
            'Update methodology based on feedback',
            project,
            dueDate: now.add(const Duration(days: 3)),
            status: TaskStatus.todo,
            priority: TaskPriority.H,
            estimatedTime: TaskTimeEstimate.M,
            estimatedEnergy: TaskEnergyEstimate.L,
          ),
          _createTask(
            'Analyze preliminary data',
            'Run statistical analysis on pilot study results',
            project,
            dueDate: now.add(const Duration(days: 7)),
            status: TaskStatus.waiting,
            priority: TaskPriority.M,
            estimatedTime: TaskTimeEstimate.M,
            estimatedEnergy: TaskEnergyEstimate.M,
          ),
          _createTask(
            'Draft research proposal',
            'Prepare document for grant submission',
            project,
            dueDate: now.add(const Duration(days: 14)),
            status: TaskStatus.todo,
            priority: TaskPriority.H,
            estimatedTime: TaskTimeEstimate.XL,
            estimatedEnergy: TaskEnergyEstimate.XL,
          ),
        ]);
        break;
    }

    return tasks;
  }

  /// Helper method to create a task with common properties
  static Task _createTask(
    String name,
    String description,
    TaskList project, {
    required DateTime dueDate,
    required TaskStatus status,
    required TaskPriority priority,
    required TaskTimeEstimate estimatedTime,
    required TaskEnergyEstimate estimatedEnergy,
  }) {
    final now = DateTime.now();
    final task = Task(
      uuid: const Uuid().v4(),
      name: name,
      description: description,
      createDate: now.subtract(const Duration(days: 2)),
      modifyDate: now,
      dueDate: dueDate,
      status: status,
      priority: priority,
      estimatedTime: estimatedTime,
      estimatedEnergy: estimatedEnergy,
    );

    task.taskList.target = project;
    return task;
  }

  /// Creates sample activities for tasks
  static List<Activity> _createSampleActivities(List<Task> tasks) {
    final activities = <Activity>[];
    final now = DateTime.now();

    // Create sample activities for some of the tasks
    for (var i = 0; i < tasks.length; i++) {
      // Only create activities for every other task
      if (i % 2 == 0) continue;

      final task = tasks[i];

      // Create a completed activity
      if (task.status.isCompleted) {
        activities.add(
          Activity(
            uuid: const Uuid().v4(),
            name: 'Working on: ${task.name}',
            startDate: now.subtract(const Duration(hours: 4)),
            endDate: now.subtract(const Duration(hours: 3)),
            kind: ActivityKind.task,
            status: ActivityStatus.completed,
          )..task.target = task,
        );
      }
      // Create an in-progress activity
      else if (task.status.isPending) {
        activities.add(
          Activity(
            uuid: const Uuid().v4(),
            name: 'Working on: ${task.name}',
            startDate: now.subtract(const Duration(minutes: 45)),
            kind: ActivityKind.task,
            status: ActivityStatus.paused,
          )..task.target = task,
        );
      }
    }

    // Add some non-task activities
    activities.addAll([
      Activity(
        uuid: const Uuid().v4(),
        name: 'Morning meditation',
        startDate: DateTime(now.year, now.month, now.day, 7, 0),
        endDate: DateTime(now.year, now.month, now.day, 7, 15),
        kind: ActivityKind.activity,
        status: ActivityStatus.completed,
      ),
      Activity(
        uuid: const Uuid().v4(),
        name: 'Team meeting',
        startDate: DateTime(now.year, now.month, now.day, 10, 0),
        endDate: DateTime(now.year, now.month, now.day, 11, 0),
        kind: ActivityKind.event,
        status: ActivityStatus.completed,
      ),
      Activity(
        uuid: const Uuid().v4(),
        name: 'Lunch break',
        startDate: DateTime(now.year, now.month, now.day, 12, 0),
        endDate: DateTime(now.year, now.month, now.day, 13, 0),
        kind: ActivityKind.activity,
        status: ActivityStatus.completed,
      ),
      Activity(
        uuid: const Uuid().v4(),
        name: 'Focus session',
        startDate: DateTime(now.year, now.month, now.day + 1, 9, 0),
        endDate: DateTime(now.year, now.month, now.day + 1, 10, 30),
        kind: ActivityKind.intervalWork,
        status: ActivityStatus.scheduled,
      ),
    ]);

    return activities;
  }

  /// Clears all sample data from the database
  static Future<void> clearAllData() async {
    final taskBox = ObjectBox.taskBox();
    final projectBox = ObjectBox.taskListBox();
    final activityBox = ObjectBox.activityBox();

    // Remove all entities
    taskBox.removeAll();
    projectBox.removeAll();
    activityBox.removeAll();

    _hasSeeded = false;
    logger.i('All data cleared from database');
  }
}
