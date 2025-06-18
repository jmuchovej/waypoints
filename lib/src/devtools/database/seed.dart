import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../flavors.dart';
import '../../infrastructure/logging.dart';
import '../../things/task/task_repository.dart';
import '../../things/project/project_repository.dart';
import '../../things/activity/activity_repository.dart';
import 'seed/tasks_learning.dart';
import 'seed/tasks_personal.dart';
import 'seed/tasks_research.dart';
import 'seed/tasks_work.dart';

class SeedDataService {
  /// Create from WidgetRef (for devtools.dart)
  factory SeedDataService.fromRef(WidgetRef ref) {
    return SeedDataService._(
      ref.read(taskRepositoryProvider),
      ref.read(projectRepositoryProvider),
      ref.read(activityRepositoryProvider),
    );
  } // Prevent concurrent seeding

  /// Create from ProviderContainer (for main.dart)
  factory SeedDataService.fromContainer(ProviderContainer container) {
    return SeedDataService._(
      container.read(taskRepositoryProvider),
      container.read(projectRepositoryProvider),
      container.read(activityRepositoryProvider),
    );
  }

  SeedDataService._(this.taskRepo, this.projectRepo, this.activityRepo);

  final TaskRepository taskRepo;
  final ProjectRepository projectRepo;
  final ActivityRepository activityRepo;

  static bool _hasSeeded = false;
  static bool _isSeeding = false;

  /// Clears all sample data from the database
  Future<void> clearAllData() async {
    dbLog.fine('Clearing all data from database...');
    // Remove all entities using repositories
    final tasksRemoved = taskRepo.removeAll();
    final projectsRemoved = projectRepo.removeAll();
    final activitiesRemoved = activityRepo.removeAll();

    _hasSeeded = false;
    dbLog.fine(
      'All data cleared from database - Tasks: $tasksRemoved, Projects: $projectsRemoved, Activities: $activitiesRemoved',
    );
  }

  /// Seeds the database with sample data if in development mode
  /// and if the database is empty
  Future<void> seedIfNeeded({bool force = false}) async {
    // Prevent concurrent seeding
    if (_isSeeding) {
      dbLog.warning('Seeding already in progress, skipping...');
      return;
    }

    // Skip if already seeded and not forced
    if (_hasSeeded && !force) {
      dbLog.fine('Data already seeded and not forcing, skipping...');
      return;
    }

    // Check if in development mode
    if (F.appFlavor != Flavor.development && !force) {
      dbLog.fine('Not in development mode and not forcing, skipping...');
      return;
    }

    _isSeeding = true;
    try {
      // Check if database is empty
      final taskCount = taskRepo.count();
      final projectCount = projectRepo.count();
      dbLog.fine('Current counts - Tasks: $taskCount, Projects: $projectCount');

      if ((taskCount == 0 && projectCount == 0) || force) {
        // Clear existing data if forcing re-seed
        if (force && (taskCount > 0 || projectCount > 0)) {
          dbLog.fine('Force re-seed: clearing existing data first');
          await clearAllData();
        }

        dbLog.fine('Starting to seed sample data...');
        await _seedSampleData();
        _hasSeeded = true;
        dbLog.fine('All data seeded to database successfully');
      } else {
        dbLog.fine('Database already contains data, skipping seed');
      }
    } catch (e, stackTrace) {
      dbLog.severe('Error during seeding: $e', e, stackTrace);
      rethrow;
    } finally {
      _isSeeding = false;
    }
  }

  /// Seeds the database with sample projects and tasks
  Future<void> _seedSampleData() async {
    PersonalArea().seed(this);
    WorkArea().seed(this);
    ResearchArea().seed(this);
    LearningArea().seed(this);
  }
}
