import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project.dart';
import '../seed.dart';
import '_seed.dart';
import 'tasks_personal_japan.dart';
import 'tasks_personal_office.dart';
import 'tasks_personal_photo.dart';

class PersonalArea extends DatabaseSeed {
  final area = ProjectEntity(
    title: AttributedText("🏠 Personal"),
    uuid: "76c14776-3c11-4e67-a7da-46c039737aa3",
  );

  @override
  void seed(SeedDataService s) {
    dbLog.fine('<seeding area="${area.titlePlainText}">');

    // Use repository which handles registration automatically
    final areaID = s.projectRepo.put(area);

    final japanTrip = JapanTripProject();
    japanTrip.seed(s);

    final homeOffice = HomeOfficeRenovationProject();
    homeOffice.seed(s);

    final photos = DigitalPhotoArchiveProject();
    photos.seed(s);

    // Get the area again with proper ID and add children
    final savedArea = s.projectRepo.get(areaID)!;
    savedArea.children.addAll(
      [
        japanTrip.project.uuid,
        homeOffice.project.uuid,
        photos.project.uuid,
      ].map((u) => s.projectRepo.byUUID(u)!.asThingEntity()),
    );
    s.projectRepo.put(savedArea);
  }
}
