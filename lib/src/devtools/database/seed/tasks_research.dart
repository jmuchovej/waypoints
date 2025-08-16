import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project.entity.dart';
import '../seed.dart';
import '_seed.dart';
import 'tasks_research_nsf.dart';
import 'tasks_research_icml.dart';
import 'tasks_research_cognitive.dart';

class ResearchArea extends DatabaseSeed {
  final seedArea = ProjectEntity(
    title: AttributedText("🧠 Research"),
    uuid: "976087f7-97e8-4185-86f8-f3b2412554ea",
  );

  @override
  Future<void> seed(SeedDataService s) async {
    waypointsDBSeedLog.fine('<seeding area="${seedArea.title.toPlainText()}">');

    final area = await s.projectRepo.putAsync(seedArea);

    final nsf = NSFGrantProposalProject();
    await nsf.seed(s);

    final icml = ICML2025Project();
    await icml.seed(s);

    final cognitive = CognitiveLoadProject();
    await cognitive.seed(s);

    area.thing.childrenRef.addAll(
      [
        nsf.seedProject.uuid,
        icml.seedProject.uuid,
        cognitive.seedProject.uuid,
      ].map((u) => s.projectRepo.byUUID(u)!.thing),
    );
    await s.projectRepo.putAsync(area);
  }
}
