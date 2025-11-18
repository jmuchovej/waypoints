import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/foundation.dart';
import '../../../things/project/project.entity.dart';
import '../seed.dart';
import '_seed.dart';

class DigitalPhotoArchiveProject extends DatabaseSeed {
  final seedProject = ProjectEntity(
    title: AttributedText("Digital Photo Archive"),
    uuid: "0a4e7ceb-6dac-4859-9376-22f8d60520dd",
  );

  @override
  Future<void> seed(SeedDataService s) async {
    waypointsDBSeedLog.finer(
      '  <seeding project="${seedProject.title.toPlainText()}">',
    );
    final project = await s.projectRepo.putAsync(seedProject);

    final scopeTask = await s.taskRepo.putAsync(buildProjectScopeTask());
    final timelineTask = await s.taskRepo.putAsync(buildProjectTimelineTask());

    project.thing.childrenRef.addAll([scopeTask, timelineTask].things);
    await s.projectRepo.putAsync(project);
  }
}
