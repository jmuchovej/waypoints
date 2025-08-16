import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project.entity.dart';
import '../seed.dart';
import '_seed.dart';
import 'tasks_work_api.dart';
import 'tasks_work_launch.dart';
import 'tasks_work_onboarding.dart';

class WorkArea extends DatabaseSeed {
  final seedArea = ProjectEntity(
    title: AttributedText("💼 Work"),
    uuid: "25416666-bec7-4c32-8d50-f406be2f7ae6",
  );

  @override
  Future<void> seed(SeedDataService s) async {
    waypointsDBSeedLog.fine('<seeding area="${seedArea.title.toPlainText()}">');

    final area = await s.projectRepo.putAsync(seedArea);

    final api = APIv2MigrationProject();
    await api.seed(s);

    final launch = Q1LaunchProject();
    await launch.seed(s);

    final onboarding = TeamOnboardingProject();
    await onboarding.seed(s);

    area.thing.childrenRef.addAll(
      [
        api.seedProject.uuid,
        launch.seedProject.uuid,
        onboarding.seedProject.uuid,
      ].map((u) => s.projectRepo.byUUID(u)!.thing),
    );
    await s.projectRepo.putAsync(area);
  }
}
