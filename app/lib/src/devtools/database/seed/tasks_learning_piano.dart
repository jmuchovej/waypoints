import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/block/block.entity.dart';
import '../../../things/foundation.dart';
import '../../../things/project/project.entity.dart';
import '../../../things/task/task.entity.dart';
import '../seed.dart';
import '_seed.dart';

class PianoChopinEduteOp10No4Project extends DatabaseSeed {
  final seedProject = ProjectEntity(
    title: AttributedText("Piano: Chopin Étude Op.10 No.4"),
    uuid: "49b1f46f-5152-419b-b7d5-eeffdcb048af",
  );

  // Project description blocks
  final seedProjectHeaderBlock = BlockEntity(
    uuid: "piano-proj-header-001",
    nodeType: "header1",
    content: AttributedText("Chopin Étude Op.10 No.4 in C# minor"),
  );

  final seedProjectDetailsBlock = BlockEntity(
    uuid: "piano-proj-details-002",
    nodeType: "paragraph",
    content: AttributedText(
      "Composer: Frédéric Chopin | Difficulty: Advanced (Grade 8+) | Target Tempo: ♩ = 152 BPM | Duration: ~2 minutes",
    ),
  );

  final seedAnalysisHeaderBlock = BlockEntity(
    uuid: "piano-proj-analysis-003",
    nodeType: "header2",
    content: AttributedText("Musical Analysis"),
  );

  final seedAnalysisBlock = BlockEntity(
    uuid: "piano-proj-analysis-004",
    nodeType: "paragraph",
    content: AttributedText(
      "This étude focuses on chromatic passages and rapid scalar runs, building dexterity in both hands. Known for its technical demands and musical expressiveness.",
    ),
  );

  final seedObjectivesHeaderBlock = BlockEntity(
    uuid: "piano-proj-objectives-005",
    nodeType: "header2",
    content: AttributedText("Learning Objectives"),
  );

  final seedObjectivesListBlock = BlockEntity(
    uuid: "piano-proj-objectives-006",
    nodeType: "list_item",
    content: AttributedText("Master chromatic scale technique at speed"),
  );

  final seedObjectivesListBlock2 = BlockEntity(
    uuid: "piano-proj-objectives-007",
    nodeType: "list_item",
    content: AttributedText("Develop independence between hands"),
  );

  final seedObjectivesListBlock3 = BlockEntity(
    uuid: "piano-proj-objectives-008",
    nodeType: "list_item",
    content: AttributedText("Build stamina for sustained rapid passages"),
  );

  final seedScheduleHeaderBlock = BlockEntity(
    uuid: "piano-proj-schedule-009",
    nodeType: "header2",
    content: AttributedText("Practice Schedule"),
  );

  final seedScheduleBlock = BlockEntity(
    uuid: "piano-proj-schedule-010",
    nodeType: "paragraph",
    content: AttributedText(
      "Daily practice: 45-60 minutes | Focus areas: Measures 1-8 (exposition), measures 25-32 (climax) | Metronome work: Start at ♩ = 100, increase by 4 BPM weekly | Performance goal: Ready for spring recital (May 2025)",
    ),
  );

  // Task definitions
  final seedTechniqueTask = DevtoolsTaskDefaults.withDefaults(
    "piano-technique-01",
    AttributedText("Master chromatic scale technique"),
    priority: TaskPriority.H,
    dateDelta: const Duration(days: 14),
  );

  // Technique task description blocks
  final seedTechniqueHeaderBlock = BlockEntity(
    uuid: "piano-tech-header-011",
    nodeType: "header3",
    content: AttributedText("Chromatic Scale Foundation Work"),
  );

  final seedTechniqueExerciseBlock = BlockEntity(
    uuid: "piano-tech-exercise-012",
    nodeType: "paragraph",
    content: AttributedText(
      "Exercise Routine: Hanon exercises 39-43 (chromatic preparation), chromatic scales in parallel motion (2 octaves), practice with varying rhythms (dotted, triplets, syncopated), focus on consistent fingering (thumb on white keys only).",
    ),
  );

  final seedTechniqueGoalsBlock = BlockEntity(
    uuid: "piano-tech-goals-013",
    nodeType: "paragraph",
    content: AttributedText(
      "Daily Goals: 15 minutes slow practice (♩ = 60), 10 minutes with metronome acceleration, 5 minutes musical application to Chopin passage.",
    ),
  );

  final seedMemorizeTask = DevtoolsTaskDefaults.withDefaults(
    "piano-memorize-01",
    AttributedText("Memorize first 16 measures"),
    dateDelta: const Duration(days: 21),
  );

  // Memorization task description blocks
  final seedMemorizeApproachBlock = BlockEntity(
    uuid: "piano-mem-approach-014",
    nodeType: "header3",
    content: AttributedText("Systematic Memorization Approach"),
  );

  final seedMemoryTypesBlock = BlockEntity(
    uuid: "piano-mem-types-015",
    nodeType: "paragraph",
    content: AttributedText(
      "Memory Types: Analytical (harmonic analysis and form structure), Kinesthetic (muscle memory through slow practice), Visual (mental image of keyboard patterns), Auditory (singing inner voices and harmonies).",
    ),
  );

  final seedPerformanceTask = DevtoolsTaskDefaults.withDefaults(
    "piano-performance-01",
    AttributedText("Record performance video"),
    dateDelta: const Duration(days: 60),
    status: TaskStatus.scheduled,
  );

  // Performance task description blocks
  final seedPerformanceStandardsBlock = BlockEntity(
    uuid: "piano-perf-standards-016",
    nodeType: "paragraph",
    content: AttributedText(
      "Performance Standards: Tempo ♩ = 140 minimum (96% of target tempo), musical expression with clear phrasing and dynamics, technical accuracy (<2 errors per page), confident stage presence and posture.",
    ),
  );

  final seedRecordingSetupBlock = BlockEntity(
    uuid: "piano-perf-recording-017",
    nodeType: "paragraph",
    content: AttributedText(
      "Recording Setup: Zoom H1n recorder positioned 3 feet from piano, iPhone on tripod for YouTube upload, natural lighting without harsh shadows, concert black attire for professional appearance.",
    ),
  );

  @override
  Future<void> seed(SeedDataService s) async {
    waypointsDBSeedLog.finer(
      '  <seedIng project="${seedProject.title.toPlainText()}">',
    );
    final project = await s.projectRepo.putAsync(seedProject);

    // Seed all block entities first
    final projectBlocks = [
      seedProjectHeaderBlock,
      seedProjectDetailsBlock,
      seedAnalysisHeaderBlock,
      seedAnalysisBlock,
      seedObjectivesHeaderBlock,
      seedObjectivesListBlock,
      seedObjectivesListBlock2,
      seedObjectivesListBlock3,
      seedScheduleHeaderBlock,
      seedScheduleBlock,
    ];
    for (var index = 0; index < projectBlocks.length; index++) {
      projectBlocks[index] = await s.blockRepo.putAsync(projectBlocks[index]);
    }

    final techniqueBlocks = [
      seedTechniqueHeaderBlock,
      seedTechniqueExerciseBlock,
      seedTechniqueGoalsBlock,
    ];
    for (var index = 0; index < techniqueBlocks.length; index++) {
      techniqueBlocks[index] = await s.blockRepo.putAsync(
        techniqueBlocks[index],
      );
    }

    final memorizeBlocks = [seedMemorizeApproachBlock, seedMemoryTypesBlock];
    for (var index = 0; index < memorizeBlocks.length; index++) {
      memorizeBlocks[index] = await s.blockRepo.putAsync(memorizeBlocks[index]);
    }

    final performanceBlocks = [
      seedPerformanceStandardsBlock,
      seedRecordingSetupBlock,
    ];
    for (var index = 0; index < performanceBlocks.length; index++) {
      performanceBlocks[index] = await s.blockRepo.putAsync(
        performanceBlocks[index],
      );
    }

    // Seed tasks
    final scopeTask = await s.taskRepo.putAsync(buildProjectScopeTask());
    final timelineTask = await s.taskRepo.putAsync(buildProjectTimelineTask());

    final techniqueTask = await s.taskRepo.putAsync(seedTechniqueTask);
    final memorizeTask = await s.taskRepo.putAsync(seedMemorizeTask);
    final performanceTask = await s.taskRepo.putAsync(seedPerformanceTask);

    // Set project relationships for all tasks
    waypointsDBSeedLog.fine(
      'Setting project relationships for ${project.title.toPlainText()}',
    );

    // Add block children to project
    project.thing.childrenRef.addAll(projectBlocks.things);
    project.thing.childrenRef.addAll(
      [
        scopeTask,
        timelineTask,
        techniqueTask,
        memorizeTask,
        performanceTask,
      ].things,
    );
    // Save the ThingEntity to persist the children relationships
    await s.projectRepo.putAsync(project);

    // Add block children to technique task
    techniqueTask.thing.childrenRef.addAll(techniqueBlocks.things);
    // Save the ThingEntity to persist the children relationships
    await s.taskRepo.putAsync(techniqueTask);

    // Add block children to memorize task
    memorizeTask.thing.childrenRef.addAll(memorizeBlocks.things);
    // Save the ThingEntity to persist the children relationships
    await s.taskRepo.putAsync(memorizeTask);

    // Add block children to performance task
    performanceTask.thing.childrenRef.addAll(performanceBlocks.things);
    // Save the ThingEntity to persist the children relationships
    await s.taskRepo.putAsync(performanceTask);
  }
}
