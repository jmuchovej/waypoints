import 'package:flutter/foundation.dart';
import 'package:tempo/main.dart' show obx;
import 'package:tempo/src/editor.dart';
import 'package:tempo/src/infrastructure.dart';
import 'package:uuid/uuid.dart';
import 'package:tempo/src/database/objectbox/models/task.dart';
import 'package:tempo/src/database/objectbox/models/project.dart';
import 'package:tempo/src/database/objectbox/models/activity.dart';

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
    final taskCount = obx.tasks.count();
    final projectCount = obx.projects.count();

    if ((taskCount == 0 && projectCount == 0) || force) {
      await _seedSampleData();
      _hasSeeded = true;
      devtoolsDatabaseLog.fine('All data seeded to database');
    } else {
      devtoolsDatabaseLog.fine('All data cleared from database');
    }
  }

  /// Seeds the database with sample projects and tasks
  static Future<void> _seedSampleData() async {
    // Create sample projects
    final projects = _createSampleProjects();
    final projectIds = obx.batchAddProjects(projects);
    devtoolsDatabaseLog.finer('Created ${projectIds.length} sample projects');

    // Reload projects to get their assigned IDs
    final savedProjects = obx.getProjects();

    // Create sample tasks for each project
    final allTasks = <TaskEntity>[];

    for (final project in savedProjects) {
      final tasks = _createSampleTasksForProject(project);
      allTasks.addAll(tasks);
    }

    // Save all tasks
    final taskIds = obx.batchAddTasks(allTasks);
    devtoolsDatabaseLog.finer('Created ${taskIds.length} sample tasks');

    // Create sample activities
    final activities = _createSampleActivities(allTasks);
    final activityIds = obx.batchAddActivities(activities);
    devtoolsDatabaseLog.finer(
      'Created ${activityIds.length} sample activities',
    );
  }

  /// Creates sample projects
  static List<ProjectEntity> _createSampleProjects() {
    return [
      ProjectEntity(
        uuid: const Uuid().v4(),
        title: AttributedText('Personal'),
        // description: 'Personal tasks and projects',
      ),
      ProjectEntity(
        uuid: const Uuid().v4(),
        title: AttributedText('Work'),
        // description: 'Work-related tasks and projects',
      ),
      ProjectEntity(
        uuid: const Uuid().v4(),
        title: AttributedText('Health'),
        // description: 'Health and wellness tasks',
      ),
      ProjectEntity(
        uuid: const Uuid().v4(),
        title: AttributedText('Research'),
        // description: 'Research projects and literature review',
      ),
    ];
  }

  /// Creates sample tasks for a given project
  static List<TaskEntity> _createSampleTasksForProject(ProjectEntity project) {
    final now = DateTime.now();
    final tasks = <TaskEntity>[];

    // Create different tasks based on project type
    switch (project.titlePlainText) {
      case 'Personal':
        tasks.addAll([
          _createTask(
            'Buy groceries',
            'Milk, eggs, bread, vegetables',
            project,
            dueDate: now.add(const Duration(days: 1)),
            status: TaskStatus.todo,
            priority: TaskPriority.M,
            timeEstimate: TaskTimeEstimate.S,
            energyEstimate: TaskEnergyEstimate.M,
          ),
          _createTask(
            'Clean apartment',
            'Vacuum, dust, clean bathroom',
            project,
            dueDate: now.add(const Duration(days: 3)),
            status: TaskStatus.scheduled,
            priority: TaskPriority.L,
            timeEstimate: TaskTimeEstimate.M,
            energyEstimate: TaskEnergyEstimate.L,
          ),
          _createTask(
            'Plan weekend trip',
            'Research destinations, book accommodations',
            project,
            dueDate: now.add(const Duration(days: 5)),
            status: TaskStatus.started,
            priority: TaskPriority.H,
            timeEstimate: TaskTimeEstimate.M,
            energyEstimate: TaskEnergyEstimate.M,
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
            timeEstimate: TaskTimeEstimate.L,
            energyEstimate: TaskEnergyEstimate.L,
          ),
          _createTask(
            'Review team members code',
            'Code review for the new feature implementation',
            project,
            dueDate: now.add(const Duration(days: 2)),
            status: TaskStatus.waiting,
            priority: TaskPriority.M,
            timeEstimate: TaskTimeEstimate.M,
            energyEstimate: TaskEnergyEstimate.M,
          ),
          _createTask(
            'Prepare for client meeting',
            'Create agenda and gather materials',
            project,
            dueDate: now.add(const Duration(days: 1)),
            status: TaskStatus.todo,
            priority: TaskPriority.H,
            timeEstimate: TaskTimeEstimate.S,
            energyEstimate: TaskEnergyEstimate.M,
          ),
          _createTask(
            'Update documentation',
            'Add new API endpoints to the documentation',
            project,
            dueDate: now.subtract(const Duration(days: 2)),
            status: TaskStatus.complete,
            priority: TaskPriority.L,
            timeEstimate: TaskTimeEstimate.M,
            energyEstimate: TaskEnergyEstimate.S,
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
            timeEstimate: TaskTimeEstimate.S,
            energyEstimate: TaskEnergyEstimate.M,
          ),
          _createTask(
            'Schedule annual checkup',
            'Call doctor\'s office for appointment',
            project,
            dueDate: now.add(const Duration(days: 14)),
            status: TaskStatus.todo,
            priority: TaskPriority.M,
            timeEstimate: TaskTimeEstimate.XS,
            energyEstimate: TaskEnergyEstimate.XS,
          ),
          _createTask(
            'Meal prep for the week',
            'Plan meals and prepare ingredients',
            project,
            dueDate: now.add(const Duration(days: 2)),
            status: TaskStatus.scheduled,
            priority: TaskPriority.M,
            timeEstimate: TaskTimeEstimate.M,
            energyEstimate: TaskEnergyEstimate.M,
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
            timeEstimate: TaskTimeEstimate.L,
            energyEstimate: TaskEnergyEstimate.L,
          ),
          _createTask(
            'Revise experiment design',
            'Update methodology based on feedback',
            project,
            dueDate: now.add(const Duration(days: 3)),
            status: TaskStatus.todo,
            priority: TaskPriority.H,
            timeEstimate: TaskTimeEstimate.M,
            energyEstimate: TaskEnergyEstimate.L,
          ),
          _createTask(
            'Analyze preliminary data',
            'Run statistical analysis on pilot study results',
            project,
            dueDate: now.add(const Duration(days: 7)),
            status: TaskStatus.waiting,
            priority: TaskPriority.M,
            timeEstimate: TaskTimeEstimate.M,
            energyEstimate: TaskEnergyEstimate.M,
          ),
          _createTask(
            'Draft research proposal',
            'Prepare document for grant submission',
            project,
            dueDate: now.add(const Duration(days: 14)),
            status: TaskStatus.todo,
            priority: TaskPriority.H,
            timeEstimate: TaskTimeEstimate.XL,
            energyEstimate: TaskEnergyEstimate.XL,
          ),
        ]);
        break;
    }

    return tasks;
  }

  /// Helper method to create a task with common properties
  static TaskEntity _createTask(
    String title,
    String notes,
    ProjectEntity project, {
    required DateTime dueDate,
    required TaskStatus status,
    required TaskPriority priority,
    required TaskTimeEstimate timeEstimate,
    required TaskEnergyEstimate energyEstimate,
  }) {
    final now = DateTime.now();
    final task = TaskEntity(
      uuid: const Uuid().v4(),
      title: AttributedText(title),
      // notes: Delta()..insert(notes),
      createDate: now.subtract(const Duration(days: 2)),
      modifyDate: now,
      dueDate: dueDate,
      status: status,
      priority: priority,
      timeEstimate: timeEstimate,
      energyEstimate: energyEstimate,
    );

    task.project.target = project;
    return task;
  }

  /// Creates sample activities for tasks
  static List<ActivityEntity> _createSampleActivities(List<TaskEntity> tasks) {
    final activities = <ActivityEntity>[];
    final now = DateTime.now();

    // Create sample activities for some of the tasks
    for (var i = 0; i < tasks.length; i++) {
      // Only create activities for every other task
      if (i % 2 == 0) continue;

      final task = tasks[i];

      // Create a completed activity
      if (task.status.isCompleted) {
        activities.add(
          ActivityEntity(
            uuid: const Uuid().v4(),
            title: 'Working on: ${task.title}',
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
          ActivityEntity(
            uuid: const Uuid().v4(),
            title: 'Working on: ${task.title}',
            startDate: now.subtract(const Duration(minutes: 45)),
            kind: ActivityKind.task,
            status: ActivityStatus.paused,
          )..task.target = task,
        );
      }
    }

    // Add some non-task activities
    activities.addAll([
      ActivityEntity(
        uuid: const Uuid().v4(),
        title: 'Morning meditation',
        startDate: DateTime(now.year, now.month, now.day, 7, 0),
        endDate: DateTime(now.year, now.month, now.day, 7, 15),
        kind: ActivityKind.activity,
        status: ActivityStatus.completed,
      ),
      ActivityEntity(
        uuid: const Uuid().v4(),
        title: 'Team meeting',
        startDate: DateTime(now.year, now.month, now.day, 10, 0),
        endDate: DateTime(now.year, now.month, now.day, 11, 0),
        kind: ActivityKind.event,
        status: ActivityStatus.completed,
      ),
      ActivityEntity(
        uuid: const Uuid().v4(),
        title: 'Lunch break',
        startDate: DateTime(now.year, now.month, now.day, 12, 0),
        endDate: DateTime(now.year, now.month, now.day, 13, 0),
        kind: ActivityKind.activity,
        status: ActivityStatus.completed,
      ),
      ActivityEntity(
        uuid: const Uuid().v4(),
        title: 'Focus session',
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
    // Remove all entities
    obx.tasks.removeAll();
    obx.projects.removeAll();
    obx.activities.removeAll();

    _hasSeeded = false;
    devtoolsDatabaseLog.fine('All data cleared from database');
  }
}
