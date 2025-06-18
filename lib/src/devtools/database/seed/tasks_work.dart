import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project.dart';
import '../seed.dart';
import '_seed.dart';
import 'tasks_work_api.dart';
import 'tasks_work_launch.dart';
import 'tasks_work_onboarding.dart';

class WorkArea extends DatabaseSeed {
  final area = ProjectEntity(
    title: AttributedText("💼 Work"),
    uuid: "25416666-bec7-4c32-8d50-f406be2f7ae6",
  );

  @override
  void seed(SeedDataService s) {
    dbLog.fine('<seeding area="${area.titlePlainText}">');

    // Use repository which handles registration automatically
    final areaID = s.projectRepo.put(area);

    final api = APIv2MigrationProject();
    api.seed(s);

    final launch = Q1LaunchProject();
    launch.seed(s);

    final onboarding = TeamOnboardingProject();
    onboarding.seed(s);

    // Get the area again with proper ID and add children
    final savedArea = s.projectRepo.get(areaID)!;
    savedArea.children.addAll(
      [
        api.project.uuid,
        launch.project.uuid,
        onboarding.project.uuid,
      ].map((u) => s.projectRepo.byUUID(u)!.asThingEntity()),
    );
    s.projectRepo.put(savedArea);
  }
}
