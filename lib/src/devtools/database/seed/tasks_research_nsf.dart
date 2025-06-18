import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project.dart';
import '../seed.dart';
import '_seed.dart';

class NSFGrantProposalProject implements DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("NSF Grant Proposal"),
    uuid: "1cd7570b-faa3-4a15-97db-53b64ffc135d",
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
