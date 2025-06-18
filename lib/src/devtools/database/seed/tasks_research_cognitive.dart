import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project.dart';
import '../seed.dart';
import '_seed.dart';

class CognitiveLoadProject implements DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("Cognitive Load Study"),
    uuid: "1522e2c7-4176-4bac-9f67-5060796b99e4",
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
