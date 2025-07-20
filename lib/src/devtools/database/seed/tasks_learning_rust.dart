import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/foundation.dart';
import '../../../things/project/project.entity.dart';
import '../../../things/task/task.entity.dart';
import '../../../things/thing.entity.dart';
import '../seed.dart';
import '_seed.dart';

class RustProgrammingCoursePoject extends DatabaseSeed {
  final seedProject = ProjectEntity(
    title: AttributedText("Rust Programming Course"),
    uuid: "266006cb-a740-430f-8eee-83d68719b5bf",
  );

  final seedReadBookTask = DevtoolsTaskDefaults.withDefaults(
    "a7dc41d9-0472-4b15-9396-2a8b58e24300",
    AttributedText('Complete "The Rust Programming Language" book'),
    status: TaskStatus.workInProgress,
    dateDelta: const Duration(days: 30),
  );

  final seedReadCh01to03Task = DevtoolsTaskDefaults.withDefaults(
    "4933bff2-0da5-4729-a8ff-2e4da9a7622d",
    AttributedText("Ch 1-3: Getting Started, Guessing Game"),
    status: TaskStatus.complete,
  );

  final seedReadCh04to06Task = DevtoolsTaskDefaults.withDefaults(
    "1b017ab6-1870-4223-b36c-5699560f124c",
    AttributedText("Ch 4-6: Ownership, Structs, Enums"),
    status: TaskStatus.complete,
  );

  final seedReadCh07to09Task = DevtoolsTaskDefaults.withDefaults(
    "854c799e-c64f-4f57-a0c1-0925a6b811f9",
    AttributedText("Ch 7-9: Modules, Collections, Error Handling"),
    status: TaskStatus.workInProgress,
  );

  final seedReadCh10to12Task = DevtoolsTaskDefaults.withDefaults(
    "670cb06d-37a5-462e-b86b-e234999c3fa1",
    AttributedText("Ch 10-12: Generics, Testing, CLI Project"),
  );

  final seedPracticeCLITask = DevtoolsTaskDefaults.withDefaults(
    "b9933326-7082-4bfb-b552-ffe4ca96886a",
    AttributedText("Build practice project: Task manager CLI"),
    dateDelta: const Duration(days: 45),
  );

  final seedDesignDataStructuresTask = DevtoolsTaskDefaults.withDefaults(
    "34a9b5cd-ed4c-4299-b626-a170fcba6fa9",
    AttributedText("Design data structures for tasks"),
  );

  final seedImplementCRUDTask = DevtoolsTaskDefaults.withDefaults(
    "494035c5-a7c2-4f65-8e3d-4a07f3147e13",
    AttributedText("Implement CRUD operations"),
  );

  final seedAddSQLiteTask = DevtoolsTaskDefaults.withDefaults(
    "bd07ed6d-3c74-41d8-9d5e-468f4473c7c4",
    AttributedText("Add SQLite persistence"),
  );

  final seedCreateCLIWithClapTask = DevtoolsTaskDefaults.withDefaults(
    "fd5ee10c-382d-4d13-9180-211293bd975e",
    AttributedText("Create CLI with `clap`"),
  );

  @override
  Future<void> seed(SeedDataService s) async {
    tempoDBSeedLog.finer(
      '  <seeding project="${seedProject.title.toPlainText()}">',
    );
    final project = await s.projectRepo.putAsync(seedProject);

    // Seed tasks sequentially to avoid UUID conflicts
    final readBookTask = await s.taskRepo.putAsync(seedReadBookTask);
    final readCh01to03Task = await s.taskRepo.putAsync(seedReadCh01to03Task);
    final readCh04to06Task = await s.taskRepo.putAsync(seedReadCh04to06Task);
    final readCh07to09Task = await s.taskRepo.putAsync(seedReadCh07to09Task);
    final readCh10to12Task = await s.taskRepo.putAsync(seedReadCh10to12Task);

    final practiceCLITask = await s.taskRepo.putAsync(seedPracticeCLITask);
    final designDataStructuresTask = await s.taskRepo.putAsync(seedDesignDataStructuresTask);
    final implementCRUDTask = await s.taskRepo.putAsync(seedImplementCRUDTask);
    final addSQLiteTask = await s.taskRepo.putAsync(seedAddSQLiteTask);
    final createCLIWithClapTask = await s.taskRepo.putAsync(seedCreateCLIWithClapTask);

    // Set up relationships
    readBookTask.thing.childrenRef.addAll([
      readCh01to03Task,
      readCh04to06Task,
      readCh07to09Task,
      readCh10to12Task,
    ].things);
    await s.taskRepo.putAsync(readBookTask);

    practiceCLITask.thing.childrenRef.addAll([
      designDataStructuresTask,
      implementCRUDTask,
      addSQLiteTask,
      createCLIWithClapTask,
    ].things);
    await s.taskRepo.putAsync(practiceCLITask);

    project.thing.childrenRef.addAll([
      readBookTask,
      practiceCLITask,
    ].things);
    await s.projectRepo.putAsync(project);
  }
}
