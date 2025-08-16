import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project.entity.dart';
import '../../../things/foundation.dart';
import '../seed.dart';
import '_seed.dart';

class TeamOnboardingProject extends DatabaseSeed {
  final seedProject = ProjectEntity(
    title: AttributedText("Team Onboarding Process"),
    uuid: "ae812e28-fe3b-4563-b291-fb2c20baefb1",
  );

  @override
  Future<void> seed(SeedDataService s) async {
    waypointsDBSeedLog.finer(
      '  <seeding project="${seedProject.title.toPlainText()}">',
    );
    final project = await s.projectRepo.putAsync(seedProject);

    final scopeTask = await s.taskRepo.putAsync(buildProjectScopeTask());
    final timelineTask = await s.taskRepo.putAsync(buildProjectTimelineTask());

    // Set project relationships for all tasks
    waypointsDBSeedLog.fine(
      'Setting project relationships for ${project.title.toPlainText()}',
    );
    project.thing.childrenRef.addAll([scopeTask, timelineTask].things);

    // Save the ThingEntity to persist the children relationships
    await s.projectRepo.putAsync(project);
  }
}
