import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/foundation.dart';
import '../../../things/project/project.entity.dart';
import '../seed.dart';
import '_seed.dart';

class NSFGrantProposalProject extends DatabaseSeed {
  final seedProject = ProjectEntity(
    title: AttributedText("NSF Grant Proposal"),
    uuid: "1cd7570b-faa3-4a15-97db-53b64ffc135d",
  );

  @override
  Future<void> seed(SeedDataService s) async {
    tempoDBSeedLog.finer(
      '  <seeding project="${seedProject.title.toPlainText()}">',
    );
    final project = await s.projectRepo.putAsync(seedProject);

    final scopeTask = await s.taskRepo.putAsync(buildProjectScopeTask());
    final timelineTask = await s.taskRepo.putAsync(buildProjectTimelineTask());

    project.thing.childrenRef.addAll([scopeTask, timelineTask].things);
    await s.projectRepo.putAsync(project);
  }
}
