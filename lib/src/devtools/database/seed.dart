import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../flavors.dart';
import '../../infrastructure/logging.dart';
import '../../objectbox/services/thing_registry.dart';
import '../../objectbox/repositories/task_repository.dart';
import '../../objectbox/repositories/project_repository.dart';
import '../../objectbox/repositories/activity_repository.dart';
import '../../objectbox/repositories/block_repository.dart';
import 'seed/tasks_learning.dart';
import 'seed/tasks_personal.dart';
import 'seed/tasks_research.dart';
import 'seed/tasks_work.dart';

class SeedDataService {
  /// Create from WidgetRef (for devtools.dart)
  factory SeedDataService.fromRef(WidgetRef ref) {
    return SeedDataService._(
      ref.read(thingRegistryProvider),
      ref.read(taskRepositoryProvider),
      ref.read(projectRepositoryProvider),
      ref.read(activityRepositoryProvider),
      ref.read(blockRepositoryProvider),
      ref.read(flavorProvider),
    );
  } // Prevent concurrent seeding

  /// Create from ProviderContainer (for main.dart)
  factory SeedDataService.fromContainer(ProviderContainer container) {
    return SeedDataService._(
      container.read(thingRegistryProvider),
      container.read(taskRepositoryProvider),
      container.read(projectRepositoryProvider),
      container.read(activityRepositoryProvider),
      container.read(blockRepositoryProvider),
      container.read(flavorProvider),
    );
  }

  SeedDataService._(
    this.thingService,
    this.taskRepo,
    this.projectRepo,
    this.activityRepo,
    this.blockRepo,
    this.flavor,
  );

  final ThingRegistry thingService;
  final TaskRepository taskRepo;
  final ProjectRepository projectRepo;
  final ActivityRepository activityRepo;
  final BlockRepository blockRepo;
  final Flavor flavor;

  static bool _hasSeeded = false;
  static bool _isSeeding = false;

  /// Clears all sample data from the database
  Future<void> clearAllData() async {
    tempoDBSeedLog.fine('Clearing all data from database...');
    // Remove all entities using repositories
    final tasksRemoved = taskRepo.removeAll();
    final projectsRemoved = projectRepo.removeAll();
    final activitiesRemoved = activityRepo.removeAll();
    final blocksRemoved = blockRepo.removeAll();

    _hasSeeded = false;
    tempoDBSeedLog.fine(
      'All data cleared from database - Tasks: $tasksRemoved, Projects: $projectsRemoved, Activities: $activitiesRemoved, Blocks: $blocksRemoved',
    );
  }

  /// Seeds the database with sample data if in development mode
  /// and if the database is empty
  ///
  /// TODO: PERFORMANCE - Seeding currently blocks the UI thread
  /// This may be due to excessive tempoDBSeedLog.ging or synchronous database operations.
  /// Consider:
  /// 1. Running seeding in an isolate
  /// 2. Reducing tempoDBSeedLog.ging verbosity during seeding
  /// 3. Using batch operations where possible
  /// 4. Adding progress indicators for long-running seeds
  Future<void> seedIfNeeded({bool force = false}) async {
    // Prevent concurrent seeding
    if (_isSeeding) {
      tempoDBSeedLog.warning('Seeding already in progress, skipping...');
      return;
    }

    // Skip if already seeded and not forced
    if (_hasSeeded && !force) {
      tempoDBSeedLog.fine('Data already seeded and not forcing, skipping...');
      return;
    }

    // Check if in development mode
    if (!flavor.isDevelopment && !force) {
      tempoDBSeedLog.fine(
        'Not in development mode and not forcing, skipping...',
      );
      return;
    }

    _isSeeding = true;
    try {
      // Check if database is empty
      final taskCount = taskRepo.count();
      final projectCount = projectRepo.count();
      tempoDBSeedLog.fine(
        'Current counts - Tasks: $taskCount, Projects: $projectCount',
      );

      if ((taskCount == 0 && projectCount == 0) || force) {
        // Clear existing data if forcing re-seed
        if (force && (taskCount > 0 || projectCount > 0)) {
          tempoDBSeedLog.fine('Force re-seed: clearing existing data first');
          await clearAllData();
        }

        tempoDBSeedLog.info('Starting to seed sample data...');
        await _seedSampleData();
        _hasSeeded = true;
        tempoDBSeedLog.info('All data seeded to database successfully');
      } else {
        tempoDBSeedLog.fine('Database already contains data, skipping seed');
      }
    } catch (e, stackTrace) {
      tempoDBSeedLog.severe('Error during seeding: $e', e, stackTrace);
      rethrow;
    } finally {
      _isSeeding = false;
    }
  }

  /// Seeds the database with sample projects and tasks
  Future<void> _seedSampleData() async {
    await PersonalArea().seed(this);
    await WorkArea().seed(this);
    await ResearchArea().seed(this);
    await LearningArea().seed(this);
  }
}
