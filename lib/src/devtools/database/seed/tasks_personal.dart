import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project.entity.dart';
import '../seed.dart';
import '_seed.dart';
import 'tasks_personal_japan.dart';
import 'tasks_personal_office.dart';
import 'tasks_personal_photo.dart';

class PersonalArea extends DatabaseSeed {
  final seedArea = ProjectEntity(
    title: AttributedText("🏠 Personal"),
    uuid: "76c14776-3c11-4e67-a7da-46c039737aa3",
  );

  @override
  Future<void> seed(SeedDataService s) async {
    tempoDBSeedLog.fine('<seeding area="${seedArea.title.toPlainText()}">');

    // Use repository which handles registration automatically
    final area = await s.projectRepo.putAsync(seedArea);

    final japanTrip = JapanTripProject();
    await japanTrip.seed(s);

    final homeOffice = HomeOfficeRenovationProject();
    await homeOffice.seed(s);

    final photos = DigitalPhotoArchiveProject();
    await photos.seed(s);

    // Get the area again with proper ID and add children
    area.thing.childrenRef.addAll(
      [
        japanTrip.seedProject.uuid,
        homeOffice.seedProject.uuid,
        photos.seedProject.uuid,
      ].map((u) => s.projectRepo.byUUID(u)!.thing),
    );
    // Save the ThingEntity to persist the children relationships
    await s.projectRepo.putAsync(area);
  }
}
