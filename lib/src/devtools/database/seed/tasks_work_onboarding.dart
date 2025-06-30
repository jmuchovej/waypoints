import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project_entity.dart';
import '../seed.dart';
import '_seed.dart';

class TeamOnboardingProject implements DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("Team Onboarding Process"),
    uuid: "ae812e28-fe3b-4563-b291-fb2c20baefb1",
  );

  @override
  void seed(SeedDataService s) {
    dbLog.finer('  <seeding project="${project.title.toPlainText()}">');
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
