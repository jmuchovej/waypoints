import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project_entity.dart';
import '../../../things/task/task_entity.dart';
import '../../../things/block/block_entity.dart';
import '../seed.dart';
import '_seed.dart';

class JapanTripProject implements DatabaseSeed {
  final project = ProjectEntity(
    uuid: "85cfc98f-2772-4b21-85bd-f6afae99a4db",
    title: AttributedText("Japan Trip 2025"),
  );

  // Project description blocks
  final projectHeaderBlock = BlockEntity(
    uuid: "japan-proj-header-001",
    nodeType: "header1",
    content: AttributedText("Japan Cultural Immersion Trip 2025"),
  );

  final projectDetailsBlock = BlockEntity(
    uuid: "japan-proj-details-002",
    nodeType: "paragraph", 
    content: AttributedText("Travel Dates: April 15-25, 2025 (Cherry blossom season!) | Duration: 10 days | Travelers: 2 adults | Budget: \$8,000 total (\$4,000 per person)"),
  );

  final objectivesHeaderBlock = BlockEntity(
    uuid: "japan-proj-objectives-003",
    nodeType: "header2",
    content: AttributedText("Trip Objectives"),
  );

  final objectivesListBlock = BlockEntity(
    uuid: "japan-proj-objectives-004",
    nodeType: "list_item",
    content: AttributedText("Experience traditional and modern Japanese culture"),
  );

  final objectivesListBlock2 = BlockEntity(
    uuid: "japan-proj-objectives-005", 
    nodeType: "list_item",
    content: AttributedText("Visit during peak cherry blossom season"),
  );

  final objectivesListBlock3 = BlockEntity(
    uuid: "japan-proj-objectives-006",
    nodeType: "list_item",
    content: AttributedText("Sample authentic cuisine across different regions"),
  );

  final focusHeaderBlock = BlockEntity(
    uuid: "japan-proj-focus-007",
    nodeType: "header2",
    content: AttributedText("Cultural Focus Areas"),
  );

  final focusBlock = BlockEntity(
    uuid: "japan-proj-focus-008",
    nodeType: "paragraph",
    content: AttributedText("Traditional: Temple visits, tea ceremony, ryokan stay | Modern: TeamLab exhibitions, Akihabara tech culture | Culinary: Sushi, ramen, kaiseki dining, street food | Nature: Cherry blossoms, bamboo groves, gardens"),
  );

  final philosophyBlock = BlockEntity(
    uuid: "japan-proj-philosophy-009",
    nodeType: "blockquote",
    content: AttributedText("Travel Philosophy: Slow travel with deep cultural immersion rather than rushing through attractions. Quality over quantity."),
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

  // Tokyo task description blocks
  final tokyoHeaderBlock = BlockEntity(
    uuid: "japan-tokyo-header-010",
    nodeType: "header3",
    content: AttributedText("Tokyo Itinerary: Modern Culture & Technology"),
  );

  final tokyoDay1Block = BlockEntity(
    uuid: "japan-tokyo-day1-011",
    nodeType: "paragraph",
    content: AttributedText("Day 1 - Arrival & Shibuya: Land at Narita, take Skyliner to city, check into Shibuya hotel, explore crossing, dinner at Ichiran Ramen (first authentic meal)."),
  );

  final tokyoDay2Block = BlockEntity(
    uuid: "japan-tokyo-day2-012", 
    nodeType: "paragraph",
    content: AttributedText("Day 2 - Traditional Meets Modern: Morning at Meiji Shrine and Harajuku fashion streets, afternoon at TeamLab Borderless (pre-booked timed entry), evening izakaya experience in Golden Gai."),
  );

  final tokyoExperiencesBlock = BlockEntity(
    uuid: "japan-tokyo-experiences-013",
    nodeType: "paragraph",
    content: AttributedText("Must-Do Experiences: Shibuya Crossing at rush hour, TeamLab digital art installation, cherry blossom picnic with sake, authentic sushi at Tsukiji."),
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

    // Seed project description blocks
    final projectBlockIDs = s.blockRepo.putMany([
      projectHeaderBlock,
      projectDetailsBlock,
      objectivesHeaderBlock,
      objectivesListBlock,
      objectivesListBlock2,
      objectivesListBlock3,
      focusHeaderBlock,
      focusBlock,
      philosophyBlock,
    ]);

    // Seed Tokyo task description blocks
    final tokyoBlockIDs = s.blockRepo.putMany([
      tokyoHeaderBlock,
      tokyoDay1Block,
      tokyoDay2Block,
      tokyoExperiencesBlock,
    ]);

    // Flights task (and subtasks)
    final [flightID, flightOptionsID, lodgingID, railpassID] = s.taskRepo
        .putMany([flightTask, flightOptionsTask, lodgingTask, railpassTask]);

    final savedFlightTask = s.taskRepo.get(flightID)!;
    savedFlightTask.children.addAll(
      s.taskRepo
          .getMany([flightOptionsID, lodgingID, railpassID])
          .map((t) => t!.asThingEntity()),
    );
    s.taskRepo.put(savedFlightTask);

    // Plan itinerary (and subtasks)
    final [itineraryID, tokyoID, osakaID, kyotoID] = s.taskRepo.putMany([
      itineraryTask,
      tokyoTask,
      osakaTask,
      kyotoTask,
    ]);

    final savedItineraryTask = s.taskRepo.get(itineraryID)!;
    savedItineraryTask.children.addAll(
      s.taskRepo
          .getMany([tokyoID, osakaID, kyotoID])
          .map((t) => t!.asThingEntity()),
    );
    s.taskRepo.put(savedItineraryTask);

    // Add block children to Tokyo task
    final savedTokyoTask = s.taskRepo.get(tokyoID)!;
    savedTokyoTask.children.addAll(
      s.blockRepo.getMany(tokyoBlockIDs).map((b) => b!.asThingEntity())
    );
    s.taskRepo.put(savedTokyoTask);

    // Add blocks and tasks to project
    final savedProject = s.projectRepo.get(projectID)!;
    savedProject.children.addAll(
      s.blockRepo.getMany(projectBlockIDs).map((b) => b!.asThingEntity())
    );
    savedProject.children.addAll(
      s.taskRepo
          .getMany([flightID, itineraryID])
          .map((t) => t!.asThingEntity()),
    );
    s.projectRepo.put(savedProject);
  }
}
