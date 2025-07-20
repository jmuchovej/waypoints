import 'package:jiffy/jiffy.dart';

import '../../../infrastructure/logging.dart';
import '../../../editor/editor.dart';
import '../../../things/foundation.dart';
import '../../../things/project/project.entity.dart';
import '../../../things/task/task.entity.dart';
import '../../../things/block/block.entity.dart';
import '../seed.dart';
import '_seed.dart';

class JapanTripProject extends DatabaseSeed {
  // Calculate next cherry blossom season (April)
  late final DateTime tripStart = () {
    final now = DateTime.now();
    final nextApril = DateTime(now.year, 4, 15);

    // If we're past April this year, plan for next year
    if (now.isAfter(DateTime(now.year, 4, 25))) {
      return DateTime(now.year + 1, 4, 15);
    }
    // If we're within 3 months of the trip, push to next year
    else if (Jiffy.parseFromDateTime(
          nextApril,
        ).diff(Jiffy.parseFromDateTime(now), unit: Unit.month) <
        3) {
      return DateTime(now.year + 1, 4, 15);
    }
    return nextApril;
  }();

  late final DateTime tripEnd = tripStart.add(const Duration(days: 10));
  late final int tripYear = tripStart.year;

  late final seedProject = ProjectEntity(
    uuid: "85cfc98f-2772-4b21-85bd-f6afae99a4db",
    title: AttributedText("Japan Trip $tripYear"),
    startDate: tripStart,
    endDate: tripEnd,
  );

  // Project description blocks
  late final seedProjectHeaderBlock = BlockEntity(
    uuid: "japan-proj-header-001",
    nodeType: "header1",
    content: AttributedText("Japan Cultural Immersion Trip $tripYear"),
  );

  late final seedProjectDetailsBlock = BlockEntity(
    uuid: "japan-proj-details-002",
    nodeType: "paragraph",
    content: AttributedText(
      "Travel Dates: ${Jiffy.parseFromDateTime(tripStart).format(pattern: 'MMMM d')}-${Jiffy.parseFromDateTime(tripEnd).format(pattern: 'd, yyyy')} (Cherry blossom season!)",
    ),
  );

  final seedProjectDetailsBlock2 = BlockEntity(
    uuid: "japan-proj-details-002a",
    nodeType: "list_item",
    content: AttributedText("Duration: 10 days"),
  );

  final seedProjectDetailsBlock3 = BlockEntity(
    uuid: "japan-proj-details-002b",
    nodeType: "list_item",
    content: AttributedText("Travelers: 2 adults"),
  );

  final seedProjectDetailsBlock4 = BlockEntity(
    uuid: "japan-proj-details-002c",
    nodeType: "list_item",
    content: AttributedText("Budget: \$8,000 total (\$4,000 per person)"),
  );

  final seedObjectivesHeaderBlock = BlockEntity(
    uuid: "japan-proj-objectives-003",
    nodeType: "header2",
    content: AttributedText("Trip Objectives"),
  );

  final seedObjectivesListBlock = BlockEntity(
    uuid: "japan-proj-objectives-004",
    nodeType: "list_item",
    content: AttributedText(
      "Experience traditional and modern Japanese culture",
    ),
  );

  final seedObjectivesListBlock2 = BlockEntity(
    uuid: "japan-proj-objectives-005",
    nodeType: "list_item",
    content: AttributedText("Visit during peak cherry blossom season"),
  );

  final seedObjectivesListBlock3 = BlockEntity(
    uuid: "japan-proj-objectives-006",
    nodeType: "list_item",
    content: AttributedText(
      "Sample authentic cuisine across different regions",
    ),
  );

  final seedFocusHeaderBlock = BlockEntity(
    uuid: "japan-proj-focus-007",
    nodeType: "header2",
    content: AttributedText("Cultural Focus Areas"),
  );

  final seedFocusTraditionalBlock = BlockEntity(
    uuid: "japan-proj-focus-008a",
    nodeType: "list_item",
    content: AttributedText(
      "Traditional: Temple visits, tea ceremony, ryokan stay",
    ),
  );

  final seedFocusModernBlock = BlockEntity(
    uuid: "japan-proj-focus-008b",
    nodeType: "list_item",
    content: AttributedText(
      "Modern: TeamLab exhibitions, Akihabara tech culture",
    ),
  );

  final seedFocusCulinaryBlock = BlockEntity(
    uuid: "japan-proj-focus-008c",
    nodeType: "list_item",
    content: AttributedText(
      "Culinary: Sushi, ramen, kaiseki dining, street food",
    ),
  );

  final seedFocusNatureBlock = BlockEntity(
    uuid: "japan-proj-focus-008d",
    nodeType: "list_item",
    content: AttributedText("Nature: Cherry blossoms, bamboo groves, gardens"),
  );

  final seedPhilosophyBlock = BlockEntity(
    uuid: "japan-proj-philosophy-009",
    nodeType: "blockquote",
    content: AttributedText(
      "Travel Philosophy: Slow travel with deep cultural immersion rather than rushing through attractions. Quality over quantity.",
    ),
  );

  // Calculate task due dates relative to trip start
  late final seedFlightTask = DevtoolsTaskDefaults.withDefaults(
    '306f0136-cd56-4f26-873a-1bf5b5736be3',
    AttributedText('Book flights and accommodations'),
    dueDate:
        Jiffy.parseFromDateTime(
          tripStart,
        ).subtract(months: 2).dateTime, // 2 months before
  );

  late final seedFlightOptionsTask = DevtoolsTaskDefaults.withDefaults(
    'f2f6a1aa-3f7d-485c-a18c-577645d3a88d',
    AttributedText('Research flight options (JAL, ANA, United)'),
    dueDate:
        Jiffy.parseFromDateTime(
          tripStart,
        ).subtract(days: 75).dateTime, // 2.5 months before
    status: TaskStatus.started,
  );

  late final seedLodgingTask = DevtoolsTaskDefaults.withDefaults(
    '98c0e18d-f7d6-46e8-bd9b-ac4b9515af88',
    AttributedText('Compare hotel vs Airbnb in Tokyo, Kyoto, Osaka'),
    dueDate:
        Jiffy.parseFromDateTime(
          tripStart,
        ).subtract(days: 65).dateTime, // Just before booking
  );

  late final seedRailpassTask = DevtoolsTaskDefaults.withDefaults(
    '54f78b93-883a-4fbb-9921-6ec3e44c30e6',
    AttributedText('Book JR Pass for rail travel'),
    dueDate:
        Jiffy.parseFromDateTime(
          tripStart,
        ).subtract(weeks: 6).dateTime, // 1.5 months before
  );

  late final seedItineraryTask = DevtoolsTaskDefaults.withDefaults(
    'b1e0ade0-d4f5-4ab9-8f51-927fcf2344e7',
    AttributedText('Plan itinerary'),
    dueDate:
        Jiffy.parseFromDateTime(
          tripStart,
        ).subtract(months: 3).dateTime, // 3 months before
  );

  final seedTokyoTask = DevtoolsTaskDefaults.withDefaults(
    'fb155ace-b082-4412-8c1a-30f1865606fc',
    AttributedText('Tokyo (5 days): Shibuya, Harajuku, Akihabara, TeamLab'),
  );

  // Tokyo task description blocks
  final seedTokyoHeaderBlock = BlockEntity(
    uuid: "japan-tokyo-header-010",
    nodeType: "header3",
    content: AttributedText(
      "Tokyo Itinerary: Modern Culture & Technology",
    ),
  );

  final seedTokyoDay1Block = BlockEntity(
    uuid: "japan-tokyo-day1-011",
    nodeType: "paragraph",
    content: AttributedText(
      "Day 1 - Arrival & Shibuya: Land at Narita, take Skyliner to city, check into Shibuya hotel, explore crossing, dinner at Ichiran Ramen (first authentic meal).",
    ),
  );

  final seedTokyoDay2Block = BlockEntity(
    uuid: "japan-tokyo-day2-012",
    nodeType: "paragraph",
    content: AttributedText(
      "Day 2 - Traditional Meets Modern: Morning at Meiji Shrine and Harajuku fashion streets, afternoon at TeamLab Borderless (pre-booked timed entry), evening izakaya experience in Golden Gai.",
    ),
  );

  final seedTokyoExperiencesBlock = BlockEntity(
    uuid: "japan-tokyo-experiences-013",
    nodeType: "paragraph",
    content: AttributedText(
      "Must-Do Experiences: Shibuya Crossing at rush hour, TeamLab digital art installation, cherry blossom picnic with sake, authentic sushi at Tsukiji.",
    ),
  );
  final seedKyotoTask = DevtoolsTaskDefaults.withDefaults(
    '2f478226-85ad-4bd8-8c10-e07b55b4912e',
    AttributedText('Kyoto (3 days): Temples, Arashiyama, Gion'),
  );
  final seedOsakaTask = DevtoolsTaskDefaults.withDefaults(
    'fbf7442f-5be7-405e-83ab-0ada07fe6fcc',
    AttributedText('Osaka (2 days): Castle, Dtonobori, Day trip to Nara'),
  );

  @override
  Future<void> seed(SeedDataService s) async {
    tempoDBSeedLog.finer(
      '  <seeding project="${seedProject.title.toPlainText()}">',
    );
    final project = await s.projectRepo.putAsync(seedProject);

    // Seed project description blocks
    final List<BlockEntity> projectBlocks = [];
    for (final block in [
      seedProjectHeaderBlock,
      seedProjectDetailsBlock,
      seedProjectDetailsBlock2,
      seedProjectDetailsBlock3,
      seedProjectDetailsBlock4,
      seedObjectivesHeaderBlock,
      seedObjectivesListBlock,
      seedObjectivesListBlock2,
      seedObjectivesListBlock3,
      seedFocusHeaderBlock,
      seedFocusTraditionalBlock,
      seedFocusModernBlock,
      seedFocusCulinaryBlock,
      seedFocusNatureBlock,
      seedPhilosophyBlock,
    ]) {
      projectBlocks.add(await s.blockRepo.putAsync(block));
    }

    // Seed Tokyo task description blocks
    final List<BlockEntity> tokyoBlocks = [];
    for (final block in [
      seedTokyoHeaderBlock,
      seedTokyoDay1Block,
      seedTokyoDay2Block,
      seedTokyoExperiencesBlock,
    ]) {
      tokyoBlocks.add(await s.blockRepo.putAsync(block));
    }

    // Flights task (and subtasks)
    final flightTask = await s.taskRepo.putAsync(seedFlightTask);
    final flightOptionsTask = await s.taskRepo.putAsync(seedFlightOptionsTask);
    final lodgingTask = await s.taskRepo.putAsync(seedLodgingTask);
    final railpassTask = await s.taskRepo.putAsync(seedRailpassTask);

    flightTask.thing.childrenRef.addAll([flightOptionsTask, lodgingTask, railpassTask].things);
    tempoDBSeedLog.finest('flightTask.children => ${flightTask.thing.children}');
    await s.taskRepo.putAsync(flightTask);

    // Plan itinerary (and subtasks)
    final itineraryTask = await s.taskRepo.putAsync(seedItineraryTask);
    final tokyoTask = await s.taskRepo.putAsync(seedTokyoTask);
    final osakaTask = await s.taskRepo.putAsync(seedOsakaTask);
    final kyotoTask = await s.taskRepo.putAsync(seedKyotoTask);

    itineraryTask.thing.childrenRef.addAll([tokyoTask, osakaTask, kyotoTask].things);
    await s.taskRepo.putAsync(itineraryTask);

    // Add block children to Tokyo task
    tokyoTask.thing.childrenRef.addAll(tokyoBlocks.things);
    await s.taskRepo.putAsync(tokyoTask);

    // Add blocks and tasks to project
    project.thing.childrenRef.addAll([...projectBlocks, flightTask, itineraryTask].things);
    await s.projectRepo.putAsync(project);
  }
}
