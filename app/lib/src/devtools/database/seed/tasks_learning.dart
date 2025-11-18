import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project.entity.dart';
import '../seed.dart';
import '_seed.dart';
import 'tasks_learning_piano.dart';
import 'tasks_learning_rust.dart';
import 'tasks_learning_spanish.dart';

class LearningArea extends DatabaseSeed {
  final seedArea = ProjectEntity(
    title: AttributedText("🎯 Learning"),
    uuid: "f3d0d057-573f-4071-849a-f5fbedf320c4",
  );

  @override
  Future<void> seed(SeedDataService s) async {
    waypointsDBSeedLog.fine('<seeding area="${seedArea.title.toPlainText()}">');

    // Use repository which handles registration automatically
    final area = await s.projectRepo.putAsync(seedArea);

    final rust = RustProgrammingCoursePoject();
    await rust.seed(s);

    final piano = PianoChopinEduteOp10No4Project();
    await piano.seed(s);

    final spanish = B2SpanishCertificationProject();
    await spanish.seed(s);

    // Add children to area
    area.thing.childrenRef.addAll(
      [
        rust.seedProject.uuid,
        piano.seedProject.uuid,
        spanish.seedProject.uuid,
      ].map((String u) => s.projectRepo.byUUID(u)!.thing),
    );
    // Save the ThingEntity to persist the children relationships
    await s.projectRepo.putAsync(area);
  }
}
