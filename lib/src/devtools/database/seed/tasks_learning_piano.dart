import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project_entity.dart';
import '../../../things/task/task_entity.dart';
import '../../../things/block/block_entity.dart';
import '../seed.dart';
import '_seed.dart';

class PianoChopinEduteOp10No4Project implements DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("Piano: Chopin Étude Op.10 No.4"),
    uuid: "49b1f46f-5152-419b-b7d5-eeffdcb048af",
  );

  // Project description blocks
  final projectHeaderBlock = BlockEntity(
    uuid: "piano-proj-header-001",
    nodeType: "header1",
    content: AttributedText("Chopin Étude Op.10 No.4 in C# minor"),
  );

  final projectDetailsBlock = BlockEntity(
    uuid: "piano-proj-details-002", 
    nodeType: "paragraph",
    content: AttributedText("Composer: Frédéric Chopin | Difficulty: Advanced (Grade 8+) | Target Tempo: ♩ = 152 BPM | Duration: ~2 minutes"),
  );

  final analysisHeaderBlock = BlockEntity(
    uuid: "piano-proj-analysis-003",
    nodeType: "header2",
    content: AttributedText("Musical Analysis"),
  );

  final analysisBlock = BlockEntity(
    uuid: "piano-proj-analysis-004",
    nodeType: "paragraph", 
    content: AttributedText("This étude focuses on chromatic passages and rapid scalar runs, building dexterity in both hands. Known for its technical demands and musical expressiveness."),
  );

  final objectivesHeaderBlock = BlockEntity(
    uuid: "piano-proj-objectives-005",
    nodeType: "header2",
    content: AttributedText("Learning Objectives"),
  );

  final objectivesListBlock = BlockEntity(
    uuid: "piano-proj-objectives-006",
    nodeType: "list_item",
    content: AttributedText("Master chromatic scale technique at speed"),
  );

  final objectivesListBlock2 = BlockEntity(
    uuid: "piano-proj-objectives-007", 
    nodeType: "list_item",
    content: AttributedText("Develop independence between hands"),
  );

  final objectivesListBlock3 = BlockEntity(
    uuid: "piano-proj-objectives-008",
    nodeType: "list_item", 
    content: AttributedText("Build stamina for sustained rapid passages"),
  );

  final scheduleHeaderBlock = BlockEntity(
    uuid: "piano-proj-schedule-009",
    nodeType: "header2",
    content: AttributedText("Practice Schedule"),
  );

  final scheduleBlock = BlockEntity(
    uuid: "piano-proj-schedule-010",
    nodeType: "paragraph",
    content: AttributedText("Daily practice: 45-60 minutes | Focus areas: Measures 1-8 (exposition), measures 25-32 (climax) | Metronome work: Start at ♩ = 100, increase by 4 BPM weekly | Performance goal: Ready for spring recital (May 2025)"),
  );

  // Task definitions
  final techniqueTask = DevtoolsTaskDefaults.withDefaults(
    "piano-technique-01",
    AttributedText("Master chromatic scale technique"),
    priority: TaskPriority.H,
    dateDelta: const Duration(days: 14),
  );

  // Technique task description blocks
  final techniqueHeaderBlock = BlockEntity(
    uuid: "piano-tech-header-011",
    nodeType: "header3",
    content: AttributedText("Chromatic Scale Foundation Work"),
  );

  final techniqueExerciseBlock = BlockEntity(
    uuid: "piano-tech-exercise-012",
    nodeType: "paragraph",
    content: AttributedText("Exercise Routine: Hanon exercises 39-43 (chromatic preparation), chromatic scales in parallel motion (2 octaves), practice with varying rhythms (dotted, triplets, syncopated), focus on consistent fingering (thumb on white keys only)."),
  );

  final techniqueGoalsBlock = BlockEntity(
    uuid: "piano-tech-goals-013", 
    nodeType: "paragraph",
    content: AttributedText("Daily Goals: 15 minutes slow practice (♩ = 60), 10 minutes with metronome acceleration, 5 minutes musical application to Chopin passage."),
  );

  final memorizeTask = DevtoolsTaskDefaults.withDefaults(
    "piano-memorize-01", 
    AttributedText("Memorize first 16 measures"),
    dateDelta: const Duration(days: 21),
  );

  // Memorization task description blocks
  final memorizeApproachBlock = BlockEntity(
    uuid: "piano-mem-approach-014",
    nodeType: "header3", 
    content: AttributedText("Systematic Memorization Approach"),
  );

  final memoryTypesBlock = BlockEntity(
    uuid: "piano-mem-types-015",
    nodeType: "paragraph",
    content: AttributedText("Memory Types: Analytical (harmonic analysis and form structure), Kinesthetic (muscle memory through slow practice), Visual (mental image of keyboard patterns), Auditory (singing inner voices and harmonies)."),
  );

  final performanceTask = DevtoolsTaskDefaults.withDefaults(
    "piano-performance-01",
    AttributedText("Record performance video"),
    dateDelta: const Duration(days: 60),
    status: TaskStatus.scheduled,
  );

  // Performance task description blocks
  final performanceStandardsBlock = BlockEntity(
    uuid: "piano-perf-standards-016",
    nodeType: "paragraph",
    content: AttributedText("Performance Standards: Tempo ♩ = 140 minimum (96% of target tempo), musical expression with clear phrasing and dynamics, technical accuracy (<2 errors per page), confident stage presence and posture."),
  );

  final recordingSetupBlock = BlockEntity(
    uuid: "piano-perf-recording-017",
    nodeType: "paragraph", 
    content: AttributedText("Recording Setup: Zoom H1n recorder positioned 3 feet from piano, iPhone on tripod for YouTube upload, natural lighting without harsh shadows, concert black attire for professional appearance."),
  );

  @override
  void seed(SeedDataService s) {
    dbLog.finer('  <seeding project="${project.title.toPlainText()}">');
    final projectID = s.projectRepo.put(project);

    // Seed all block entities first
    final projectBlockIDs = s.blockRepo.putMany([
      projectHeaderBlock,
      projectDetailsBlock, 
      analysisHeaderBlock,
      analysisBlock,
      objectivesHeaderBlock,
      objectivesListBlock,
      objectivesListBlock2,
      objectivesListBlock3,
      scheduleHeaderBlock,
      scheduleBlock,
    ]);

    final techniqueBlockIDs = s.blockRepo.putMany([
      techniqueHeaderBlock,
      techniqueExerciseBlock,
      techniqueGoalsBlock,
    ]);

    final memorizeBlockIDs = s.blockRepo.putMany([
      memorizeApproachBlock,
      memoryTypesBlock,
    ]);

    final performanceBlockIDs = s.blockRepo.putMany([
      performanceStandardsBlock,
      recordingSetupBlock,
    ]);

    // Seed tasks
    final scopeTask = buildProjectScopeTask();
    final timelineTask = buildProjectTimelineTask();

    final [
      scopeID, 
      timelineID, 
      techniqueID, 
      memorizeID, 
      performanceID
    ] = s.taskRepo.putMany([
      scopeTask, 
      timelineTask, 
      techniqueTask, 
      memorizeTask, 
      performanceTask
    ]);

    // Add block children to project
    final savedProject = s.projectRepo.get(projectID)!;
    savedProject.children.addAll(
      s.blockRepo.getMany(projectBlockIDs).map((b) => b!.asThingEntity())
    );
    savedProject.children.addAll(
      s.taskRepo.getMany([
        scopeID, 
        timelineID, 
        techniqueID, 
        memorizeID, 
        performanceID
      ]).map((t) => t!.asThingEntity()),
    );
    s.projectRepo.put(savedProject);

    // Add block children to technique task
    final savedTechniqueTask = s.taskRepo.get(techniqueID)!;
    savedTechniqueTask.children.addAll(
      s.blockRepo.getMany(techniqueBlockIDs).map((b) => b!.asThingEntity())
    );
    s.taskRepo.put(savedTechniqueTask);

    // Add block children to memorize task
    final savedMemorizeTask = s.taskRepo.get(memorizeID)!;
    savedMemorizeTask.children.addAll(
      s.blockRepo.getMany(memorizeBlockIDs).map((b) => b!.asThingEntity())
    );
    s.taskRepo.put(savedMemorizeTask);

    // Add block children to performance task
    final savedPerformanceTask = s.taskRepo.get(performanceID)!;
    savedPerformanceTask.children.addAll(
      s.blockRepo.getMany(performanceBlockIDs).map((b) => b!.asThingEntity())
    );
    s.taskRepo.put(savedPerformanceTask);
  }
}