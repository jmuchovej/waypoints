import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/foundation.dart';
import '../../../things/project/project.entity.dart';
import '../../../things/task/task.entity.dart';
import '../seed.dart';
import '_seed.dart';

class B2SpanishCertificationProject extends DatabaseSeed {
  final seedProject = ProjectEntity(
    title: AttributedText("B2 Spanish Certification"),
    uuid: "824ef3b9-ca9a-4dee-9395-3f2cdd3faad0",
  );

  final seedWeeklyPracticeTask = DevtoolsTaskDefaults.withDefaults(
    "d705f256-c064-4eed-9037-ba3ddb479033",
    AttributedText("Weekly conversation practice"),
    dateDelta: const Duration(days: 3),
    priority: TaskPriority.H,
  );

  final seedItalkiLessonTask = DevtoolsTaskDefaults.withDefaults(
    "f7af7b6e-f873-408b-ab08-1645dc7b5181",
    AttributedText("iTalki lesson – Maria (Wed)"),
    dateDelta: const Duration(days: 2),
    status: TaskStatus.scheduled,
  );

  final seedConversationTask = DevtoolsTaskDefaults.withDefaults(
    "98c66994-6699-463e-a731-b8dc37b2f919",
    AttributedText("Conversation exchange – Carlos (Fri)"),
    dateDelta: const Duration(days: 4),
    status: TaskStatus.scheduled,
  );

  final seedCompletePracticeExamTask = DevtoolsTaskDefaults.withDefaults(
    "9f2c9911-221d-4351-bc34-6b805c52d70f",
    AttributedText("Complete DELE B2 Practice Exam"),
    dateDelta: const Duration(days: 10),
  );

  final seedReadingCompTask = DevtoolsTaskDefaults.withDefaults(
    "c3946df5-2902-40d8-991e-f2f172e4a187",
    AttributedText("Reading comprehension section"),
  );

  final seedListeningCompTask = DevtoolsTaskDefaults.withDefaults(
    "b23caeb8-3343-48bb-9ea4-24b718c8108b",
    AttributedText("Listening comprehension section"),
  );

  final seedWrittenCompTask = DevtoolsTaskDefaults.withDefaults(
    "e60a3ff5-538f-4e98-8ae3-c38b79242c38",
    AttributedText("Written expression section"),
  );

  final seedOralCompTask = DevtoolsTaskDefaults.withDefaults(
    "7de3b684-7648-49f8-bf27-168d91415d35",
    AttributedText("Oral expression section"),
  );

  @override
  Future<void> seed(SeedDataService s) async {
    tempoDBSeedLog.finer(
      '  <seeding project="${seedProject.title.toPlainText()}">',
    );
    final project = await s.projectRepo.putAsync(seedProject);

    // Weekly practice task and subtasks
    final weeklyPracticeTask = await s.taskRepo.putAsync(seedWeeklyPracticeTask);
    final italkiLessonTask = await s.taskRepo.putAsync(seedItalkiLessonTask);
    final conversationTask = await s.taskRepo.putAsync(seedConversationTask);

    weeklyPracticeTask.thing.childrenRef.addAll([italkiLessonTask, conversationTask].things);
    await s.taskRepo.putAsync(weeklyPracticeTask);

    // Complete practice exam task and subtasks
    final completePracticeExamTask = await s.taskRepo.putAsync(seedCompletePracticeExamTask);
    final readingCompTask = await s.taskRepo.putAsync(seedReadingCompTask);
    final listeningCompTask = await s.taskRepo.putAsync(seedListeningCompTask);
    final writtenCompTask = await s.taskRepo.putAsync(seedWrittenCompTask);
    final oralCompTask = await s.taskRepo.putAsync(seedOralCompTask);

    completePracticeExamTask.thing.childrenRef.addAll([
      readingCompTask, listeningCompTask, writtenCompTask, oralCompTask
    ].things);
    await s.taskRepo.putAsync(completePracticeExamTask);

    project.thing.childrenRef.addAll([weeklyPracticeTask, completePracticeExamTask].things);
    await s.projectRepo.putAsync(project);
  }
}
