import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project.dart';
import '../../../things/task/task_entity.dart';
import '../seed.dart';
import '_seed.dart';

class ICML2025Project implements DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("ML Paper Submission: ICML 2025"),
    uuid: "2a162804-05e2-4875-99dc-559bb6ac48c6",
  );

  final litReviewTask = DevtoolsTaskDefaults.withDefaults(
    "f439f063-5425-4230-bbb8-385f5b0b43f5",
    AttributedText("Literature Review"),
    dateDelta: const Duration(days: 14),
    priority: TaskPriority.H,
    status: TaskStatus.workInProgress,
  );

  final litReviewAttnTask = DevtoolsTaskDefaults.withDefaults(
    "551541de-2a34-411d-a418-d23273428006",
    AttributedText("Review: 'Attention Is All You Need' follow-up papers"),
    status: TaskStatus.complete,
  );

  final litReviewSparseAttnTask = DevtoolsTaskDefaults.withDefaults(
    "b027abc5-bb81-4ff2-b02b-2c9e4337efbd",
    AttributedText("Review: 'Recent Sparse Attention Mechanisms'"),
    status: TaskStatus.workInProgress,
  );

  final litReviewEfficientTransformersTask = DevtoolsTaskDefaults.withDefaults(
    "334f0b41-f62f-4f04-a6c0-2af9cb85dcd4",
    AttributedText("Review: 'Efficient transformers survey' (Tay et al.)"),
    status: TaskStatus.started,
  );

  final litBiliographyTask = DevtoolsTaskDefaults.withDefaults(
    "67ba259a-2201-4614-a0ad-17544d5a9330",
    AttributedText("Compile bibliography in Zotero"),
  );

  final experimentTask = DevtoolsTaskDefaults.withDefaults(
    "2d1f940e-c7f2-45a8-9550-4bb31aeb08c5",
    AttributedText("Experimental setup"),
    dateDelta: const Duration(days: 30),
    priority: TaskPriority.H,
  );

  final prepareDatasetsTask = DevtoolsTaskDefaults.withDefaults(
    "f9beb0d8-a6e6-4442-852b-ab75d39c5cb4",
    AttributedText("Prepare datasets: WikiText-103, C4, OpenWebText"),
  );

  final implementBaselinesTask = DevtoolsTaskDefaults.withDefaults(
    "9edc2fd6-5a98-477a-920d-43900ec16878",
    AttributedText("Implement baseline models"),
  );

  final deployOnGPUTask = DevtoolsTaskDefaults.withDefaults(
    "17fdd742-fee9-435e-b180-6d1c8b8fdb82",
    AttributedText("Setup GPU cluster experiments"),
  );

  final paperDraftTask = DevtoolsTaskDefaults.withDefaults(
    "406189c9-3a43-4163-a8f9-2c69d7558422",
    AttributedText("Write paper draft"),
    dateDelta: const Duration(days: 90),
  );

  final writeAbstractTask = DevtoolsTaskDefaults.withDefaults(
    "71e52aca-f9e4-4695-ab90-2fe20c91766d",
    AttributedText("Write abstract and introduction"),
  );

  final writeRelatedWorksTask = DevtoolsTaskDefaults.withDefaults(
    "bf63d3f1-193c-4371-a582-a20f9abd5047",
    AttributedText("Write related works"),
  );

  final writeMethodsTask = DevtoolsTaskDefaults.withDefaults(
    "69bb5fa4-cf9b-4db4-861b-463920161587",
    AttributedText("Write methods"),
  );

  final writeResultsTask = DevtoolsTaskDefaults.withDefaults(
    "b32b5428-cf13-4046-8382-1b394a6e0e81",
    AttributedText("Write results and discussion"),
  );

  @override
  void seed(SeedDataService s) {
    dbLog.finer('  <seeding project="${project.titlePlainText}">');
    final projectID = s.projectRepo.put(project);

    // Literature review task and subtasks
    final [
      litReviewID,
      litReviewAttnID,
      litReviewSparseAttnID,
      litReviewEfficientTransformersID,
      litBiliographyID,
    ] = s.taskRepo.putMany([
      litReviewTask,
      litReviewAttnTask,
      litReviewSparseAttnTask,
      litReviewEfficientTransformersTask,
      litBiliographyTask,
    ]);

    final savedLitReviewTask = s.taskRepo.get(litReviewID)!;
    savedLitReviewTask.children.addAll(
      s.taskRepo
          .getMany([
            litReviewAttnID,
            litReviewSparseAttnID,
            litReviewEfficientTransformersID,
            litBiliographyID,
          ])
          .map((t) => t!.asThingEntity()),
    );
    s.taskRepo.put(savedLitReviewTask);

    // Experiment task and subtasks
    final [
      experimentID,
      prepareDatasetsID,
      implementBaselinesID,
      deployOnGPUID,
    ] = s.taskRepo.putMany([
      experimentTask,
      prepareDatasetsTask,
      implementBaselinesTask,
      deployOnGPUTask,
    ]);

    final savedExperimentTask = s.taskRepo.get(experimentID)!;
    savedExperimentTask.children.addAll(
      s.taskRepo
          .getMany([prepareDatasetsID, implementBaselinesID, deployOnGPUID])
          .map((t) => t!.asThingEntity()),
    );
    s.taskRepo.put(savedExperimentTask);

    // Paper draft task and subtasks
    final [
      paperDraftID,
      writeAbstractID,
      writeRelatedWorksID,
      writeMethodsID,
      writeResultsID,
    ] = s.taskRepo.putMany([
      paperDraftTask,
      writeAbstractTask,
      writeRelatedWorksTask,
      writeMethodsTask,
      writeResultsTask,
    ]);

    final savedPaperDraftTask = s.taskRepo.get(paperDraftID)!;
    savedPaperDraftTask.children.addAll(
      s.taskRepo
          .getMany([
            writeAbstractID,
            writeRelatedWorksID,
            writeMethodsID,
            writeResultsID,
          ])
          .map((t) => t!.asThingEntity()),
    );
    s.taskRepo.put(savedPaperDraftTask);

    final savedProject = s.projectRepo.get(projectID)!;
    savedProject.children.addAll(
      s.taskRepo
          .getMany([litReviewID, experimentID, paperDraftID])
          .map((t) => t!.asThingEntity()),
    );
    s.projectRepo.put(savedProject);
  }
}
