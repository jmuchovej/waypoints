import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project.dart';
import '../../../things/task/task_entity.dart';
import '../seed.dart';
import '_seed.dart';

class JapanTripProject implements DatabaseSeed {
  final project = ProjectEntity(
    uuid: "85cfc98f-2772-4b21-85bd-f6afae99a4db",
    title: AttributedText("Japan Trip 2025"),
  );

  final flightTask = DevtoolsTaskDefaults.withDefaults(
    '306f0136-cd56-4f26-873a-1bf5b5736be3',
    AttributedText('Book flights and accomodations'),
    dateDelta: const Duration(days: 60),
  );

  final flightOptionsTask = DevtoolsTaskDefaults.withDefaults(
    'f2f6a1aa-3f7d-485c-a18c-577645d3a88d',
    AttributedText('Research flight options (JAL, ANA, United)'),
    dateDelta: const Duration(days: 45),
    status: TaskStatus.started,
  );

  final lodgingTask = DevtoolsTaskDefaults.withDefaults(
    '98c0e18d-f7d6-46e8-bd9b-ac4b9515af88',
    AttributedText('Compare hotel vs Airbnb in Tokyo, Kyoto, Osaka'),
    dateDelta: const Duration(days: 50),
  );

  final railpassTask = DevtoolsTaskDefaults.withDefaults(
    '54f78b93-883a-4fbb-9921-6ec3e44c30e6',
    AttributedText('Book JR Pass for rail travel'),
    dateDelta: const Duration(days: 55),
  );

  final itineraryTask = DevtoolsTaskDefaults.withDefaults(
    'b1e0ade0-d4f5-4ab9-8f51-927fcf2344e7',
    AttributedText('Plan itinerary'),
    dateDelta: const Duration(days: 75),
  );

  final tokyoTask = DevtoolsTaskDefaults.withDefaults(
    'fb155ace-b082-4412-8c1a-30f1865606fc',
    AttributedText('Tokyo (5 days): Shibuya, Harajuku, Akihabara, TeamLab'),
  );
  final kyotoTask = DevtoolsTaskDefaults.withDefaults(
    '2f478226-85ad-4bd8-8c10-e07b55b4912e',
    AttributedText('Kyoto (3 days): Temples, Arashiyama, Gion'),
  );
  final osakaTask = DevtoolsTaskDefaults.withDefaults(
    'fbf7442f-5be7-405e-83ab-0ada07fe6fcc',
    AttributedText('Osaka (2 days): Castle, Dtonobori, Day trip to Nara'),
  );

  @override
  void seed(SeedDataService s) {
    dbLog.finer('  <seeding project="${project.title.toPlainText()}">');
    final projectID = s.projectRepo.put(project);

    // Flights task (and subtasks)
    final [flightID, flightOptionsID, lodgingID, railpassID] = s.taskRepo
        .putMany([flightTask, flightOptionsTask, lodgingTask, railpassTask]);
    dbLog.finest(
      '    - adding `flightTask`, `flightOptionsTask`, `lodgingTask`, `railpassTask`',
    );
    dbLog.finest('id= 1; ${s.taskRepo.ts.get(1)}');
    dbLog.finest('id=89; ${s.taskRepo.ts.get(89)}');
    dbLog.finest('id=90; ${s.taskRepo.ts.get(90)}');

    final savedFlightTask = s.taskRepo.get(flightID)!;
    savedFlightTask.children.addAll(
      s.taskRepo
          .getMany([flightOptionsID, lodgingID, railpassID])
          .map((t) => t!.asThingEntity()),
    );
    dbLog.finest(savedFlightTask);
    savedFlightTask.children.applyToDb();
    dbLog.finest('    - updated `flightTask` hierarchy.');

    // Plan itinerary (and subtasks)
    final [itineraryID, tokyoID, osakaID, kyotoID] = s.taskRepo.putMany([
      itineraryTask,
      tokyoTask,
      osakaTask,
      kyotoTask,
    ]);
    dbLog.finest(
      '    - adding `itineraryTask`, `tokyoTask`, `osakaTask`, `kyotoTask`',
    );

    final savedItineraryTask = s.taskRepo.get(itineraryID)!;
    savedItineraryTask.children.addAll(
      s.taskRepo
          .getMany([tokyoID, osakaID, kyotoID])
          .map((t) => t!.asThingEntity()),
    );
    savedItineraryTask.children.applyToDb();

    final savedProject = s.projectRepo.get(projectID)!;
    savedProject.children.addAll(
      s.taskRepo
          .getMany([flightID, itineraryID])
          .map((t) => t!.asThingEntity()),
    );
    savedProject.children.applyToDb();
    dbLog.finest('    - updated `project` hierarchy!');
  }
}
