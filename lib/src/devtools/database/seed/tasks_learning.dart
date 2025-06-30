import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project_entity.dart';
import '../seed.dart';
import '_seed.dart';
import 'tasks_learning_rust.dart';
import 'tasks_learning_piano.dart';
import 'tasks_learning_spanish.dart';

class LearningArea extends DatabaseSeed {
  final area = ProjectEntity(
    title: AttributedText("🎯 Learning"),
    uuid: "f3d0d057-573f-4071-849a-f5fbedf320c4",
  );

  @override
  void seed(SeedDataService s) {
    dbLog.fine('<seeding area="${area.title.toPlainText()}">');

    // Use repository which handles registration automatically
    final areaID = s.projectRepo.put(area);

    final rust = RustProgrammingCoursePoject();
    rust.seed(s);

    final piano = PianoChopinEduteOp10No4Project();
    piano.seed(s);

    final spanish = B2SpanishCertificationProject();
    spanish.seed(s);

    // Get the area again with proper ID and add children
    final savedArea = s.projectRepo.get(areaID)!;
    savedArea.children.addAll(
      [
        rust.project.uuid,
        piano.project.uuid,
        spanish.project.uuid,
      ].map((String u) => s.projectRepo.byUUID(u)!.asThingEntity()),
    );
    s.projectRepo.put(savedArea);
  }
}
