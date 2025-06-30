import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project_entity.dart';
import '../seed.dart';
import '_seed.dart';
import 'tasks_research_nsf.dart';
import 'tasks_research_icml.dart';
import 'tasks_research_cognitive.dart';

class ResearchArea extends DatabaseSeed {
  final area = ProjectEntity(
    title: AttributedText("🧠 Research"),
    uuid: "976087f7-97e8-4185-86f8-f3b2412554ea",
  );

  @override
  void seed(SeedDataService s) {
    dbLog.fine('<seeding area="${area.title.toPlainText()}">');

    // Use repository which handles registration automatically
    final areaID = s.projectRepo.put(area);

    final nsf = NSFGrantProposalProject();
    nsf.seed(s);

    final icml = ICML2025Project();
    icml.seed(s);

    final cognitive = CognitiveLoadProject();
    cognitive.seed(s);

    // Get the area again with proper ID and add children
    final savedArea = s.projectRepo.get(areaID)!;
    savedArea.children.addAll(
      [
        nsf.project.uuid,
        icml.project.uuid,
        cognitive.project.uuid,
      ].map((u) => s.projectRepo.byUUID(u)!.asThingEntity()),
    );
    s.projectRepo.put(savedArea);
  }
}
