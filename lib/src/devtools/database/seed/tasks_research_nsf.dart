import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project.entity.dart';
import '../../../things/thing/thing.entity.dart';
import '../seed.dart';
import '_seed.dart';

class NSFGrantProposalProject extends DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("NSF Grant Proposal"),
    uuid: "1cd7570b-faa3-4a15-97db-53b64ffc135d",
  );

  @override
  void seed(SeedDataService s) {
    tempoDBSeedLog.finer(
      '  <seeding project="${project.title.toPlainText()}">',
    );
    final projectID = s.projectRepo.put(project);

    final scopeTask = buildProjectScopeTask();
    final timelineTask = buildProjectTimelineTask();

    final [scopeID, timelineID] = s.taskRepo.putMany([scopeTask, timelineTask]);

    // Set project relationships for all tasks
    final savedProject = s.projectRepo.get(projectID)!;
    tempoDBSeedLog.fine(
      'Setting project relationships for ${savedProject.title.toPlainText()}',
    );
    for (final taskID in [scopeID, timelineID]) {
      final task = s.taskRepo.get(taskID)!;
      task.project.target = savedProject;
      s.taskRepo.put(task);
      tempoDBSeedLog.fine(
        'Set project for task ${task.uuid}: ${task.title.toPlainText()} -> ${task.project.target?.title.toPlainText()}',
      );
    }
    final savedProjectThingEntity = savedProject.thing.target!;
    savedProjectThingEntity.children.addAll(
      s.taskRepo.getMany([scopeID, timelineID]).map((t) => t!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.projectRepo.store.box<ThingEntity>().put(savedProjectThingEntity);
    s.projectRepo.put(savedProject);
  }
}
