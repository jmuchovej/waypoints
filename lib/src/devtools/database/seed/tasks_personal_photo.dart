import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project.dart';
import '../seed.dart';
import '_seed.dart';

class DigitalPhotoArchiveProject implements DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("Digital Photo Archive"),
    uuid: "0a4e7ceb-6dac-4859-9376-22f8d60520dd",
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
