import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project.entity.dart';
import '../../../things/task/task.entity.dart';
import '../../../things/thing/thing.entity.dart';
import '../seed.dart';
import '_seed.dart';

class B2SpanishCertificationProject extends DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("B2 Spanish Certification"),
    uuid: "824ef3b9-ca9a-4dee-9395-3f2cdd3faad0",
  );

  final weeklyPracticeTask = DevtoolsTaskDefaults.withDefaults(
    "d705f256-c064-4eed-9037-ba3ddb479033",
    AttributedText("Weekly conversation practice"),
    dateDelta: const Duration(days: 3),
    priority: TaskPriority.H,
  );

  final italkiLessonTask = DevtoolsTaskDefaults.withDefaults(
    "f7af7b6e-f873-408b-ab08-1645dc7b5181",
    AttributedText("iTalki lesson – Maria (Wed)"),
    dateDelta: const Duration(days: 2),
    status: TaskStatus.scheduled,
  );

  final conversationTask = DevtoolsTaskDefaults.withDefaults(
    "98c66994-6699-463e-a731-b8dc37b2f919",
    AttributedText("Conversation exchange – Carlos (Fri)"),
    dateDelta: const Duration(days: 4),
    status: TaskStatus.scheduled,
  );

  final completePracticeExamTask = DevtoolsTaskDefaults.withDefaults(
    "9f2c9911-221d-4351-bc34-6b805c52d70f",
    AttributedText("Complete DELE B2 Practice Exam"),
    dateDelta: const Duration(days: 10),
  );

  final readingCompTask = DevtoolsTaskDefaults.withDefaults(
    "c3946df5-2902-40d8-991e-f2f172e4a187",
    AttributedText("Reading comprehension section"),
  );

  final listeningCompTask = DevtoolsTaskDefaults.withDefaults(
    "b23caeb8-3343-48bb-9ea4-24b718c8108b",
    AttributedText("Listening comprehension section"),
  );

  final writtenCompTask = DevtoolsTaskDefaults.withDefaults(
    "e60a3ff5-538f-4e98-8ae3-c38b79242c38",
    AttributedText("Written expression section"),
  );

  final oralCompTask = DevtoolsTaskDefaults.withDefaults(
    "7de3b684-7648-49f8-bf27-168d91415d35",
    AttributedText("Oral expression section"),
  );

  @override
  void seed(SeedDataService s) {
    tempoDBSeedLog.finer(
      '  <seeding project="${project.title.toPlainText()}">',
    );
    final projectID = s.projectRepo.put(project);

    // Weekly practice task and subtasks
    final [weeklyPracticeID, italkiLessonID, conversationID] = s.taskRepo
        .putMany([weeklyPracticeTask, italkiLessonTask, conversationTask]);

    final savedWeeklyPracticeTask = s.taskRepo.get(weeklyPracticeID)!;
    final savedWeeklyPracticeTaskThingEntity =
        savedWeeklyPracticeTask.thing.target!;
    savedWeeklyPracticeTaskThingEntity.children.addAll(
      s.taskRepo
          .getMany([italkiLessonID, conversationID])
          .map((t) => t!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(savedWeeklyPracticeTaskThingEntity);
    s.taskRepo.put(savedWeeklyPracticeTask);

    // Complete practice exam task and subtasks
    final [
      completePracticeExamID,
      readingCompID,
      listeningCompID,
      writtenCompID,
      oralCompID,
    ] = s.taskRepo.putMany([
      completePracticeExamTask,
      readingCompTask,
      listeningCompTask,
      writtenCompTask,
      oralCompTask,
    ]);

    // Set project relationships for all tasks
    final savedProject = s.projectRepo.get(projectID)!;
    tempoDBSeedLog.fine(
      'Setting project relationships for ${savedProject.title.toPlainText()}',
    );
    for (final taskID in [
      completePracticeExamID,
      readingCompID,
      listeningCompID,
      writtenCompID,
      oralCompID,
    ]) {
      final task = s.taskRepo.get(taskID)!;
      task.project.target = savedProject;
      s.taskRepo.put(task);
      tempoDBSeedLog.fine(
        'Set project for task ${task.uuid}: ${task.title.toPlainText()} -> ${task.project.target?.title.toPlainText()}',
      );
    }

    final savedCompletePracticeExamTask =
        s.taskRepo.get(completePracticeExamID)!;
    final savedCompletePracticeExamTaskThingEntity =
        savedCompletePracticeExamTask.thing.target!;
    savedCompletePracticeExamTaskThingEntity.children.addAll(
      s.taskRepo
          .getMany([readingCompID, listeningCompID, writtenCompID, oralCompID])
          .map((t) => t!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(
      savedCompletePracticeExamTaskThingEntity,
    );
    s.taskRepo.put(savedCompletePracticeExamTask);

    final savedProjectThingEntity = savedProject.thing.target!;
    savedProjectThingEntity.children.addAll(
      s.taskRepo
          .getMany([weeklyPracticeID, completePracticeExamID])
          .map((t) => t!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.projectRepo.store.box<ThingEntity>().put(savedProjectThingEntity);
    s.projectRepo.put(savedProject);
  }
}
