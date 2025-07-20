import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/foundation.dart';
import '../../../things/project/project.entity.dart';
import '../../../things/task/task.entity.dart';
import '../../../things/thing.entity.dart';
import '../seed.dart';
import '_seed.dart';

class ICML2025Project extends DatabaseSeed {
  final seedProject = ProjectEntity(
    title: AttributedText("ML Paper Submission: ICML 2025"),
    uuid: "2a162804-05e2-4875-99dc-559bb6ac48c6",
  );

  final seedLitReviewTask = DevtoolsTaskDefaults.withDefaults(
    "f439f063-5425-4230-bbb8-385f5b0b43f5",
    AttributedText("Literature Review"),
    dateDelta: const Duration(days: 14),
    priority: TaskPriority.H,
    status: TaskStatus.workInProgress,
  );

  final seedLitReviewAttnTask = DevtoolsTaskDefaults.withDefaults(
    "551541de-2a34-411d-a418-d23273428006",
    AttributedText("Review: 'Attention Is All You Need' follow-up papers"),
    status: TaskStatus.complete,
  );

  final seedLitReviewSparseAttnTask = DevtoolsTaskDefaults.withDefaults(
    "b027abc5-bb81-4ff2-b02b-2c9e4337efbd",
    AttributedText("Review: 'Recent Sparse Attention Mechanisms'"),
    status: TaskStatus.workInProgress,
  );

  final seedLitReviewEfficientTransformersTask = DevtoolsTaskDefaults.withDefaults(
    "334f0b41-f62f-4f04-a6c0-2af9cb85dcd4",
    AttributedText("Review: 'Efficient transformers survey' (Tay et al.)"),
    status: TaskStatus.started,
  );

  final seedLitBiliographyTask = DevtoolsTaskDefaults.withDefaults(
    "67ba259a-2201-4614-a0ad-17544d5a9330",
    AttributedText("Compile bibliography in Zotero"),
  );

  final seedExperimentTask = DevtoolsTaskDefaults.withDefaults(
    "2d1f940e-c7f2-45a8-9550-4bb31aeb08c5",
    AttributedText("Experimental setup"),
    dateDelta: const Duration(days: 30),
    priority: TaskPriority.H,
  );

  final seedPrepareDatasetsTask = DevtoolsTaskDefaults.withDefaults(
    "f9beb0d8-a6e6-4442-852b-ab75d39c5cb4",
    AttributedText("Prepare datasets: WikiText-103, C4, OpenWebText"),
  );

  final seedImplementBaselinesTask = DevtoolsTaskDefaults.withDefaults(
    "9edc2fd6-5a98-477a-920d-43900ec16878",
    AttributedText("Implement baseline models"),
  );

  final seedDeployOnGPUTask = DevtoolsTaskDefaults.withDefaults(
    "17fdd742-fee9-435e-b180-6d1c8b8fdb82",
    AttributedText("Setup GPU cluster experiments"),
  );

  final seedPaperDraftTask = DevtoolsTaskDefaults.withDefaults(
    "406189c9-3a43-4163-a8f9-2c69d7558422",
    AttributedText("Write paper draft"),
    dateDelta: const Duration(days: 90),
  );

  final seedWriteAbstractTask = DevtoolsTaskDefaults.withDefaults(
    "71e52aca-f9e4-4695-ab90-2fe20c91766d",
    AttributedText("Write abstract and introduction"),
  );

  final seedWriteRelatedWorksTask = DevtoolsTaskDefaults.withDefaults(
    "bf63d3f1-193c-4371-a582-a20f9abd5047",
    AttributedText("Write related works"),
  );

  final seedWriteMethodsTask = DevtoolsTaskDefaults.withDefaults(
    "69bb5fa4-cf9b-4db4-861b-463920161587",
    AttributedText("Write methods"),
  );

  final seedWriteResultsTask = DevtoolsTaskDefaults.withDefaults(
    "b32b5428-cf13-4046-8382-1b394a6e0e81",
    AttributedText("Write results and discussion"),
  );

  @override
  Future<void> seed(SeedDataService s) async {
    tempoDBSeedLog.finer(
      '  <seeding project="${seedProject.title.toPlainText()}">',
    );
    final project = await s.projectRepo.putAsync(seedProject);

    // Literature review task and subtasks
    final litReviewTask = await s.taskRepo.putAsync(seedLitReviewTask);
    final litReviewAttnTask = await s.taskRepo.putAsync(seedLitReviewAttnTask);
    final litReviewSparseAttnTask = await s.taskRepo.putAsync(seedLitReviewSparseAttnTask);
    final litReviewEfficientTransformersTask = await s.taskRepo.putAsync(seedLitReviewEfficientTransformersTask);
    final litBiliographyTask = await s.taskRepo.putAsync(seedLitBiliographyTask);

    litReviewTask.thing.childrenRef.addAll([
      litReviewAttnTask,
      litReviewSparseAttnTask,
      litReviewEfficientTransformersTask,
      litBiliographyTask,
    ].things);
    await s.taskRepo.putAsync(litReviewTask);

    // Experiment task and subtasks
    final experimentTask = await s.taskRepo.putAsync(seedExperimentTask);
    final prepareDatasetsTask = await s.taskRepo.putAsync(seedPrepareDatasetsTask);
    final implementBaselinesTask = await s.taskRepo.putAsync(seedImplementBaselinesTask);
    final deployOnGPUTask = await s.taskRepo.putAsync(seedDeployOnGPUTask);

    experimentTask.thing.childrenRef.addAll([
      prepareDatasetsTask,
      implementBaselinesTask,
      deployOnGPUTask,
    ].things);
    await s.taskRepo.putAsync(experimentTask);

    // Paper draft task and subtasks
    final paperDraftTask = await s.taskRepo.putAsync(seedPaperDraftTask);
    final writeAbstractTask = await s.taskRepo.putAsync(seedWriteAbstractTask);
    final writeRelatedWorksTask = await s.taskRepo.putAsync(seedWriteRelatedWorksTask);
    final writeMethodsTask = await s.taskRepo.putAsync(seedWriteMethodsTask);
    final writeResultsTask = await s.taskRepo.putAsync(seedWriteResultsTask);

    paperDraftTask.thing.childrenRef.addAll([
      writeAbstractTask,
      writeRelatedWorksTask,
      writeMethodsTask,
      writeResultsTask,
    ].things);
    await s.taskRepo.putAsync(paperDraftTask);

    project.thing.childrenRef.addAll([litReviewTask, experimentTask, paperDraftTask].things);
    await s.projectRepo.putAsync(project);
  }
}
