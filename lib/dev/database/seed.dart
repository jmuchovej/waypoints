import 'package:flutter/foundation.dart';
import 'package:tempo/main.dart' show obx;
import 'package:tempo/src/editor.dart';
import 'package:tempo/src/infrastructure.dart';
import 'package:uuid/uuid.dart';
import 'package:tempo/src/things/entities.dart';

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
      devtoolsDatabaseLog.fine('Database already contains data');
    }
  }

  /// Seeds the database with sample projects and tasks
  static Future<void> _seedSampleData() async {
    // Create sample areas (top-level projects that act as areas)
    final areas = _createSampleAreas();
    final areaIds = obx.batchAddProjects(areas);
    devtoolsDatabaseLog.finer('Created ${areaIds.length} sample areas');

    // Reload areas to get their assigned IDs
    final savedAreas =
        obx.getProjects().where((p) => p.parent.target == null).toList();

    // Create actionable projects under each area
    final allProjects = <ProjectEntity>[];
    final projectsByArea = <ProjectEntity, List<ProjectEntity>>{};

    for (final area in savedAreas) {
      final projects = _createProjectsForArea(area);
      allProjects.addAll(projects);
      projectsByArea[area] = projects;
    }

    final projectIds = obx.batchAddProjects(allProjects);
    devtoolsDatabaseLog.finer(
      'Created ${projectIds.length} actionable projects',
    );

    // Reload all projects to get their assigned IDs
    final savedProjects =
        obx.getProjects().where((p) => p.parent.target != null).toList();

    // Create tasks with subtasks for each project
    final allTasks = <TaskEntity>[];

    for (final project in savedProjects) {
      final tasksWithSubtasks = _createTasksForProject(project);
      allTasks.addAll(tasksWithSubtasks);
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

  /// Creates sample areas (top-level projects that act as areas in PARA)
  static List<ProjectEntity> _createSampleAreas() {
    return [
      ProjectEntity(
        uuid: const Uuid().v4(),
        title: AttributedText('🏠 Personal'),
      ),
      ProjectEntity(uuid: const Uuid().v4(), title: AttributedText('💼 Work')),
      ProjectEntity(
        uuid: const Uuid().v4(),
        title: AttributedText('🧠 Research'),
      ),
      ProjectEntity(
        uuid: const Uuid().v4(),
        title: AttributedText('🎯 Learning'),
      ),
    ];
  }

  /// Creates actionable projects for a given area
  static List<ProjectEntity> _createProjectsForArea(ProjectEntity area) {
    final projects = <ProjectEntity>[];
    final now = DateTime.now();

    switch (area.titlePlainText) {
      case '🏠 Personal':
        projects.addAll([
          ProjectEntity(
            uuid: const Uuid().v4(),
            title: AttributedText('Japan Trip 2025'),
          )..parent.target = area,
          ProjectEntity(
            uuid: const Uuid().v4(),
            title: AttributedText('Home Office Renovation'),
          )..parent.target = area,
          ProjectEntity(
            uuid: const Uuid().v4(),
            title: AttributedText('Digital Photo Archive'),
          )..parent.target = area,
        ]);
        break;

      case '💼 Work':
        projects.addAll([
          ProjectEntity(
            uuid: const Uuid().v4(),
            title: AttributedText('Q1 Product Launch'),
          )..parent.target = area,
          ProjectEntity(
            uuid: const Uuid().v4(),
            title: AttributedText('Team Onboarding Process'),
          )..parent.target = area,
          ProjectEntity(
            uuid: const Uuid().v4(),
            title: AttributedText('API v2 Migration'),
          )..parent.target = area,
        ]);
        break;

      case '🧠 Research':
        projects.addAll([
          ProjectEntity(
            uuid: const Uuid().v4(),
            title: AttributedText('ML Paper Submission - ICML 2025'),
          )..parent.target = area,
          ProjectEntity(
            uuid: const Uuid().v4(),
            title: AttributedText('Grant Proposal - NSF'),
          )..parent.target = area,
          ProjectEntity(
            uuid: const Uuid().v4(),
            title: AttributedText('Cognitive Load Study'),
          )..parent.target = area,
        ]);
        break;

      case '🎯 Learning':
        projects.addAll([
          ProjectEntity(
            uuid: const Uuid().v4(),
            title: AttributedText('Rust Programming Course'),
          )..parent.target = area,
          ProjectEntity(
            uuid: const Uuid().v4(),
            title: AttributedText('Spanish B2 Certification'),
          )..parent.target = area,
          ProjectEntity(
            uuid: const Uuid().v4(),
            title: AttributedText('Piano - Chopin Etude Op.10 No.4'),
          )..parent.target = area,
        ]);
        break;
    }

    return projects;
  }

  /// Creates tasks with subtasks for a given project
  static List<TaskEntity> _createTasksForProject(ProjectEntity project) {
    final now = DateTime.now();
    final tasks = <TaskEntity>[];

    switch (project.titlePlainText) {
      // Personal Projects
      case 'Japan Trip 2025':
        tasks.addAll(
          _createTaskHierarchy([
            _TaskTemplate(
              title: 'Book flights and accommodations',
              status: TaskStatus.todo,
              dueDate: now.add(const Duration(days: 60)),
              priority: TaskPriority.H,
              subtasks: [
                _TaskTemplate(
                  title: 'Research flight options (JAL, ANA, United)',
                  status: TaskStatus.started,
                  dueDate: now.add(const Duration(days: 45)),
                ),
                _TaskTemplate(
                  title: 'Compare hotel vs Airbnb in Tokyo, Kyoto, Osaka',
                  status: TaskStatus.todo,
                  dueDate: now.add(const Duration(days: 50)),
                ),
                _TaskTemplate(
                  title: 'Book JR Pass for rail travel',
                  status: TaskStatus.todo,
                  dueDate: now.add(const Duration(days: 55)),
                ),
              ],
            ),
            _TaskTemplate(
              title: 'Plan itinerary',
              status: TaskStatus.todo,
              dueDate: now.add(const Duration(days: 75)),
              priority: TaskPriority.M,
              subtasks: [
                _TaskTemplate(
                  title:
                      'Tokyo (5 days): Shibuya, Harajuku, Akihabara, TeamLab',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Kyoto (3 days): Temples, Arashiyama, Gion',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Osaka (2 days): Castle, Dotonbori, Day trip to Nara',
                  status: TaskStatus.todo,
                ),
              ],
            ),
          ], project),
        );
        break;

      case 'Home Office Renovation':
        tasks.addAll(
          _createTaskHierarchy([
            _TaskTemplate(
              title: 'Design and planning',
              status: TaskStatus.workInProgress,
              dueDate: now.add(const Duration(days: 14)),
              priority: TaskPriority.H,
              subtasks: [
                _TaskTemplate(
                  title: 'Measure room dimensions',
                  status: TaskStatus.complete,
                ),
                _TaskTemplate(
                  title: 'Create mood board on Pinterest',
                  status: TaskStatus.workInProgress,
                ),
                _TaskTemplate(
                  title: 'Sketch layout options',
                  status: TaskStatus.started,
                ),
              ],
            ),
            _TaskTemplate(
              title: 'Purchase furniture and equipment',
              status: TaskStatus.todo,
              dueDate: now.add(const Duration(days: 30)),
              priority: TaskPriority.M,
              subtasks: [
                _TaskTemplate(
                  title: 'Standing desk: Compare Uplift vs Fully vs Flexispot',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title:
                      'Ergonomic chair: Test Herman Miller Aeron vs Steelcase Leap',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Monitor arm for dual 27" displays',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Cable management system',
                  status: TaskStatus.todo,
                ),
              ],
            ),
          ], project),
        );
        break;

      // Work Projects
      case 'Q1 Product Launch':
        tasks.addAll(
          _createTaskHierarchy([
            _TaskTemplate(
              title: 'Complete feature development',
              status: TaskStatus.workInProgress,
              dueDate: now.subtract(const Duration(days: 2)), // Overdue!
              priority: TaskPriority.H,
              subtasks: [
                _TaskTemplate(
                  title: 'Implement user authentication flow',
                  status: TaskStatus.complete,
                ),
                _TaskTemplate(
                  title: 'Build dashboard analytics view',
                  status: TaskStatus.workInProgress,
                ),
                _TaskTemplate(
                  title: 'Add export functionality',
                  status: TaskStatus.started,
                ),
                _TaskTemplate(
                  title: 'Integrate payment processing',
                  status: TaskStatus.todo,
                ),
              ],
            ),
            _TaskTemplate(
              title: 'Prepare marketing materials',
              status: TaskStatus.started,
              dueDate: now.add(const Duration(days: 7)),
              priority: TaskPriority.H,
              subtasks: [
                _TaskTemplate(
                  title: 'Write product announcement blog post',
                  status: TaskStatus.started,
                ),
                _TaskTemplate(
                  title: 'Create demo video script',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Design email campaign templates',
                  status: TaskStatus.todo,
                ),
              ],
            ),
          ], project),
        );
        break;

      case 'API v2 Migration':
        tasks.addAll(
          _createTaskHierarchy([
            _TaskTemplate(
              title: 'Document current API endpoints',
              status: TaskStatus.complete,
              dueDate: now.subtract(const Duration(days: 7)),
              priority: TaskPriority.M,
              subtasks: [
                _TaskTemplate(
                  title: 'List all v1 endpoints with parameters',
                  status: TaskStatus.complete,
                ),
                _TaskTemplate(
                  title: 'Identify deprecated endpoints',
                  status: TaskStatus.complete,
                ),
                _TaskTemplate(
                  title: 'Map v1 to v2 endpoint changes',
                  status: TaskStatus.complete,
                ),
              ],
            ),
            _TaskTemplate(
              title: 'Update client libraries',
              status: TaskStatus.todo,
              dueDate: now.add(const Duration(days: 21)),
              priority: TaskPriority.H,
              subtasks: [
                _TaskTemplate(
                  title: 'Update JavaScript SDK',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Update Python SDK',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Update mobile SDKs (iOS/Android)',
                  status: TaskStatus.todo,
                ),
              ],
            ),
          ], project),
        );
        break;

      // Research Projects
      case 'ML Paper Submission - ICML 2025':
        tasks.addAll(
          _createTaskHierarchy([
            _TaskTemplate(
              title: 'Literature review',
              status: TaskStatus.workInProgress,
              dueDate: now.add(const Duration(days: 14)),
              priority: TaskPriority.H,
              subtasks: [
                _TaskTemplate(
                  title: 'Review: "Attention Is All You Need" follow-up papers',
                  status: TaskStatus.complete,
                ),
                _TaskTemplate(
                  title: 'Review: Recent sparse attention mechanisms',
                  status: TaskStatus.workInProgress,
                ),
                _TaskTemplate(
                  title: 'Review: Efficient transformers survey (Tay et al.)',
                  status: TaskStatus.started,
                ),
                _TaskTemplate(
                  title: 'Compile bibliography in Zotero',
                  status: TaskStatus.todo,
                ),
              ],
            ),
            _TaskTemplate(
              title: 'Experimental setup',
              status: TaskStatus.todo,
              dueDate: now.add(const Duration(days: 30)),
              priority: TaskPriority.H,
              subtasks: [
                _TaskTemplate(
                  title: 'Prepare datasets: WikiText-103, C4, OpenWebText',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Implement baseline models',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Set up GPU cluster experiments',
                  status: TaskStatus.todo,
                ),
              ],
            ),
            _TaskTemplate(
              title: 'Write paper draft',
              status: TaskStatus.todo,
              dueDate: now.add(const Duration(days: 90)),
              priority: TaskPriority.M,
              subtasks: [
                _TaskTemplate(
                  title: 'Abstract and introduction',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Related work section',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Methodology section',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Results and discussion',
                  status: TaskStatus.todo,
                ),
              ],
            ),
          ], project),
        );
        break;

      // Learning Projects
      case 'Rust Programming Course':
        tasks.addAll(
          _createTaskHierarchy([
            _TaskTemplate(
              title: 'Complete "The Rust Programming Language" book',
              status: TaskStatus.workInProgress,
              dueDate: now.add(const Duration(days: 30)),
              priority: TaskPriority.M,
              subtasks: [
                _TaskTemplate(
                  title: 'Ch 1-3: Getting Started, Guessing Game',
                  status: TaskStatus.complete,
                ),
                _TaskTemplate(
                  title: 'Ch 4-6: Ownership, Structs, Enums',
                  status: TaskStatus.complete,
                ),
                _TaskTemplate(
                  title: 'Ch 7-9: Modules, Collections, Error Handling',
                  status: TaskStatus.workInProgress,
                ),
                _TaskTemplate(
                  title: 'Ch 10-12: Generics, Testing, CLI Project',
                  status: TaskStatus.todo,
                ),
              ],
            ),
            _TaskTemplate(
              title: 'Build practice project: Task manager CLI',
              status: TaskStatus.todo,
              dueDate: now.add(const Duration(days: 45)),
              priority: TaskPriority.M,
              subtasks: [
                _TaskTemplate(
                  title: 'Design data structures for tasks',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Implement CRUD operations',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Add SQLite persistence',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Create CLI interface with clap',
                  status: TaskStatus.todo,
                ),
              ],
            ),
          ], project),
        );
        break;

      case 'Spanish B2 Certification':
        tasks.addAll(
          _createTaskHierarchy([
            _TaskTemplate(
              title: 'Weekly conversation practice',
              status: TaskStatus.scheduled,
              dueDate: now.add(const Duration(days: 3)),
              priority: TaskPriority.H,
              recurrence: 'FREQ=WEEKLY;BYDAY=MO,WE,FR',
              subtasks: [
                _TaskTemplate(
                  title: 'iTalki lesson - María (Wednesday)',
                  status: TaskStatus.scheduled,
                  dueDate: now.add(const Duration(days: 2)),
                ),
                _TaskTemplate(
                  title: 'Conversation exchange - Carlos (Friday)',
                  status: TaskStatus.scheduled,
                  dueDate: now.add(const Duration(days: 4)),
                ),
              ],
            ),
            _TaskTemplate(
              title: 'Complete DELE B2 practice exam',
              status: TaskStatus.todo,
              dueDate: now.add(const Duration(days: 10)),
              priority: TaskPriority.M,
              subtasks: [
                _TaskTemplate(
                  title: 'Reading comprehension section',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Listening comprehension section',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Written expression section',
                  status: TaskStatus.todo,
                ),
                _TaskTemplate(
                  title: 'Oral expression practice',
                  status: TaskStatus.todo,
                ),
              ],
            ),
          ], project),
        );
        break;

      // Add catch-all for other projects
      default:
        tasks.addAll([
          _createTask(
            'Define project scope and objectives',
            'Create clear goals and success metrics',
            project,
            dueDate: now.add(const Duration(days: 7)),
            status: TaskStatus.todo,
            priority: TaskPriority.H,
            timeEstimate: TaskTimeEstimate.M,
            energyEstimate: TaskEnergyEstimate.M,
          ),
          _createTask(
            'Create project timeline',
            'Break down into milestones and deliverables',
            project,
            dueDate: now.add(const Duration(days: 14)),
            status: TaskStatus.todo,
            priority: TaskPriority.M,
            timeEstimate: TaskTimeEstimate.M,
            energyEstimate: TaskEnergyEstimate.M,
          ),
        ]);
    }

    return tasks;
  }

  /// Helper to create a task hierarchy from templates
  static List<TaskEntity> _createTaskHierarchy(
    List<_TaskTemplate> templates,
    ProjectEntity project,
  ) {
    final tasks = <TaskEntity>[];

    for (final template in templates) {
      final parentTask = _createTaskFromTemplate(template, project);
      tasks.add(parentTask);

      // Create subtasks
      for (final subtaskTemplate in template.subtasks) {
        final subtask = _createTaskFromTemplate(subtaskTemplate, project);
        subtask.parent.target = parentTask;
        tasks.add(subtask);
      }
    }

    return tasks;
  }

  /// Helper to create a task from a template
  static TaskEntity _createTaskFromTemplate(
    _TaskTemplate template,
    ProjectEntity project,
  ) {
    final now = DateTime.now();
    return _createTask(
      template.title,
      template.notes,
      project,
      dueDate: template.dueDate ?? now.add(const Duration(days: 7)),
      status: template.status,
      priority: template.priority,
      timeEstimate: template.timeEstimate,
      energyEstimate: template.energyEstimate,
      recurrence: template.recurrence,
    );
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
    String? recurrence,
  }) {
    final now = DateTime.now();
    final task = TaskEntity(
      uuid: const Uuid().v4(),
      title: AttributedText(title),
      // Created a week ago
      createDate: now.subtract(Duration(days: 7)),
      // Modified based on due date
      modifyDate: now.subtract(
        Duration(hours: (now.difference(dueDate).inHours.abs() * 0.3).round()),
      ),
      dueDate: dueDate,
      status: status,
      priority: priority,
      timeEstimate: timeEstimate,
      energyEstimate: energyEstimate,
      recurrence: recurrence,
    );

    // Set dates based on status
    switch (status) {
      case TaskStatus.scheduled:
        task.scheduledDate = dueDate.subtract(const Duration(days: 1));
        break;
      case TaskStatus.started:
      case TaskStatus.workInProgress:
        task.startDate = now.subtract(const Duration(days: 2));
        break;
      case TaskStatus.complete:
        task.startDate = now.subtract(const Duration(days: 3));
        task.endDate = now.subtract(const Duration(days: 1));
        break;
      case TaskStatus.waiting:
        task.waitDate = now;
        task.untilDate = now.add(const Duration(days: 3));
        break;
      default:
        break;
    }

    task.project.target = project;
    return task;
  }

  /// Creates sample activities for tasks
  static List<ActivityEntity> _createSampleActivities(List<TaskEntity> tasks) {
    final activities = <ActivityEntity>[];
    final now = DateTime.now();

    // Create activities for tasks that are in progress or completed
    final activeTasks =
        tasks
            .where(
              (t) =>
                  t.status == TaskStatus.workInProgress ||
                  t.status == TaskStatus.complete ||
                  t.status == TaskStatus.started,
            )
            .toList();

    for (final task in activeTasks.take(10)) {
      // Limit to 10 activities
      if (task.status == TaskStatus.complete) {
        // Create a completed activity
        activities.add(
          ActivityEntity(
            uuid: const Uuid().v4(),
            title: 'Completed: ${task.title}',
            startDate: task.startDate ?? now.subtract(const Duration(hours: 2)),
            endDate: task.endDate ?? now.subtract(const Duration(hours: 1)),
            kind: ActivityKind.task,
            status: ActivityStatus.completed,
          )..task.target = task,
        );
      } else {
        // Create an in-progress or paused activity
        activities.add(
          ActivityEntity(
            uuid: const Uuid().v4(),
            title: 'Working on: ${task.title}',
            startDate: now.subtract(const Duration(minutes: 45)),
            kind: ActivityKind.task,
            status:
                task.status == TaskStatus.workInProgress
                    ? ActivityStatus.active
                    : ActivityStatus.paused,
          )..task.target = task,
        );
      }
    }

    // Add some non-task activities for today
    activities.addAll([
      ActivityEntity(
        uuid: const Uuid().v4(),
        title: '🧘 Morning meditation',
        startDate: DateTime(now.year, now.month, now.day, 7, 0),
        endDate: DateTime(now.year, now.month, now.day, 7, 20),
        kind: ActivityKind.activity,
        status: ActivityStatus.completed,
        tags: ['wellness', 'routine'],
      ),
      ActivityEntity(
        uuid: const Uuid().v4(),
        title: '☕ Coffee break & email check',
        startDate: DateTime(now.year, now.month, now.day, 9, 0),
        endDate: DateTime(now.year, now.month, now.day, 9, 15),
        kind: ActivityKind.activity,
        status: ActivityStatus.completed,
        tags: ['break', 'routine'],
      ),
      ActivityEntity(
        uuid: const Uuid().v4(),
        title: '🏃 Lunch run',
        startDate: DateTime(now.year, now.month, now.day, 12, 0),
        endDate: DateTime(now.year, now.month, now.day, 12, 45),
        kind: ActivityKind.activity,
        status: ActivityStatus.completed,
        tags: ['exercise', 'wellness'],
      ),
      ActivityEntity(
        uuid: const Uuid().v4(),
        title: '📅 Weekly team standup',
        startDate: DateTime(now.year, now.month, now.day, 10, 0),
        endDate: DateTime(now.year, now.month, now.day, 10, 30),
        kind: ActivityKind.event,
        status: ActivityStatus.completed,
        tags: ['meeting', 'work'],
      ),
      ActivityEntity(
        uuid: const Uuid().v4(),
        title: '🎯 Deep work session',
        startDate: DateTime(now.year, now.month, now.day, 14, 0),
        endDate: DateTime(now.year, now.month, now.day, 16, 0),
        kind: ActivityKind.intervalWork,
        status:
            now.hour >= 16 ? ActivityStatus.completed : ActivityStatus.active,
        tags: ['focus', 'work'],
      ),
    ]);

    // Add some scheduled activities for tomorrow
    activities.addAll([
      ActivityEntity(
        uuid: const Uuid().v4(),
        title: '💻 Code review session',
        startDate: DateTime(now.year, now.month, now.day + 1, 9, 30),
        endDate: DateTime(now.year, now.month, now.day + 1, 10, 30),
        kind: ActivityKind.event,
        status: ActivityStatus.scheduled,
        tags: ['work', 'collaboration'],
      ),
      ActivityEntity(
        uuid: const Uuid().v4(),
        title: '📚 Reading: "Atomic Habits"',
        startDate: DateTime(now.year, now.month, now.day + 1, 20, 0),
        endDate: DateTime(now.year, now.month, now.day + 1, 21, 0),
        kind: ActivityKind.activity,
        status: ActivityStatus.scheduled,
        tags: ['learning', 'personal'],
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

/// Template for creating tasks with optional subtasks
class _TaskTemplate {
  final String title;
  final String notes;
  final TaskStatus status;
  final DateTime? dueDate;
  final TaskPriority priority;
  final TaskTimeEstimate timeEstimate;
  final TaskEnergyEstimate energyEstimate;
  final String? recurrence;
  final List<_TaskTemplate> subtasks;

  _TaskTemplate({
    required this.title,
    this.notes = '',
    this.status = TaskStatus.todo,
    this.dueDate,
    this.priority = TaskPriority.M,
    this.timeEstimate = TaskTimeEstimate.M,
    this.energyEstimate = TaskEnergyEstimate.M,
    this.recurrence,
    this.subtasks = const [],
  });
}
