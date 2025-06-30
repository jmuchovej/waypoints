import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project_entity.dart';
import '../../../things/task/task_entity.dart';
import '../seed.dart';
import '_seed.dart';

class RustProgrammingCoursePoject implements DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("Rust Programming Course"),
    uuid: "266006cb-a740-430f-8eee-83d68719b5bf",
  );

  final readBookTask = DevtoolsTaskDefaults.withDefaults(
    "a7dc41d9-0472-4b15-9396-2a8b58e24300",
    AttributedText('Complete "The Rust Programming Language" book'),
    status: TaskStatus.workInProgress,
    dateDelta: const Duration(days: 30),
  );

  final readCh01to03Task = DevtoolsTaskDefaults.withDefaults(
    "4933bff2-0da5-4729-a8ff-2e4da9a7622d",
    AttributedText("Ch 1-3: Getting Started, Guessing Game"),
    status: TaskStatus.complete,
  );

  final readCh04to06Task = DevtoolsTaskDefaults.withDefaults(
    "1b017ab6-1870-4223-b36c-5699560f124c",
    AttributedText("Ch 4-6: Ownership, Structs, Enums"),
    status: TaskStatus.complete,
  );

  final readCh07to09Task = DevtoolsTaskDefaults.withDefaults(
    "854c799e-c64f-4f57-a0c1-0925a6b811f9",
    AttributedText("Ch 7-9: Modules, Collections, Error Handling"),
    status: TaskStatus.workInProgress,
  );

  final readCh10to12Task = DevtoolsTaskDefaults.withDefaults(
    "670cb06d-37a5-462e-b86b-e234999c3fa1",
    AttributedText("Ch 10-12: Generics, Testing, CLI Project"),
  );

  final practiceCLITask = DevtoolsTaskDefaults.withDefaults(
    "b9933326-7082-4bfb-b552-ffe4ca96886a",
    AttributedText("Build practice project: Task manager CLI"),
    dateDelta: const Duration(days: 45),
  );

  final designDataStructuresTask = DevtoolsTaskDefaults.withDefaults(
    "34a9b5cd-ed4c-4299-b626-a170fcba6fa9",
    AttributedText("Design data structures for tasks"),
  );

  final implementCRUDTask = DevtoolsTaskDefaults.withDefaults(
    "494035c5-a7c2-4f65-8e3d-4a07f3147e13",
    AttributedText("Implement CRUD operations"),
  );

  final addSQLiteTask = DevtoolsTaskDefaults.withDefaults(
    "bd07ed6d-3c74-41d8-9d5e-468f4473c7c4",
    AttributedText("Add SQLite persistence"),
  );

  final createCLIWithClapTask = DevtoolsTaskDefaults.withDefaults(
    "fd5ee10c-382d-4d13-9180-211293bd975e",
    AttributedText("Create CLI with `clap`"),
  );

  @override
  void seed(SeedDataService s) {
    dbLog.finer('  <seeding project="${project.title.toPlainText()}">');
    final projectID = s.projectRepo.put(project);

    // Read book task (and subtasks)
    final [
      readBookID,
      readCh01to03ID,
      readCh04to06ID,
      readCh07to09ID,
      readCh10to12ID,
    ] = s.taskRepo.putMany([
      readBookTask,
      readCh01to03Task,
      readCh04to06Task,
      readCh07to09Task,
      readCh10to12Task,
    ]);

    final savedReadBookTask = s.taskRepo.get(readBookID)!;
    savedReadBookTask.children.addAll(
      s.taskRepo
          .getMany([
            readCh01to03ID,
            readCh04to06ID,
            readCh07to09ID,
            readCh10to12ID,
          ])
          .map((t) => t!.asThingEntity()),
    );
    s.taskRepo.put(savedReadBookTask);

    // Practice CLI task (and subtasks)
    final [practiceCLIID, designID, crudID, sqliteID, clapID] = s.taskRepo
        .putMany([
          practiceCLITask,
          designDataStructuresTask,
          implementCRUDTask,
          addSQLiteTask,
          createCLIWithClapTask,
        ]);

    final savedPracticeCLITask = s.taskRepo.get(practiceCLIID)!;
    savedPracticeCLITask.children.addAll(
      s.taskRepo
          .getMany([designID, crudID, sqliteID, clapID])
          .map((t) => t!.asThingEntity()),
    );
    s.taskRepo.put(savedPracticeCLITask);

    final savedProject = s.projectRepo.get(projectID)!;
    savedProject.children.addAll(
      s.taskRepo
          .getMany([readBookID, practiceCLIID])
          .map((t) => t!.asThingEntity()),
    );
    s.projectRepo.put(savedProject);
  }
}
