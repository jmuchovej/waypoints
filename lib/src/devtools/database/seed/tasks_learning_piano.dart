import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project.dart';
import '../seed.dart';
import '_seed.dart';

class PianoChopinEduteOp10No4Project implements DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("Piano: Chopin Edute Op.10 No.4"),
    uuid: "49b1f46f-5152-419b-b7d5-eeffdcb048af",
  );

  @override
  void seed(SeedDataService s) {
    dbLog.finer('  <seeding project="${project.titlePlainText}">');
    final projectID = s.projectRepo.put(project);

    final scopeTask = buildProjectScopeTask();
    final timelineTask = buildProjectTimelineTask();

    final [scopeID, timelineID] = s.taskRepo.putMany([scopeTask, timelineTask]);

    final savedProject = s.projectRepo.get(projectID)!;
    savedProject.children.addAll(
      s.taskRepo.getMany([scopeID, timelineID]).map((t) => t!.asThingEntity()),
    );
    s.projectRepo.put(savedProject);
  }
}
