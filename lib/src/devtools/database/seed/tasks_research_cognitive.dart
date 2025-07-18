import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project.entity.dart';
import '../../../things/task/task.entity.dart';
import '../../../things/block/block.entity.dart';
import '../../../things/thing/thing.entity.dart';
import '../seed.dart';
import '_seed.dart';

class CognitiveLoadProject extends DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("Cognitive Load in Multi-Modal Interfaces"),
    uuid: "1522e2c7-4176-4bac-9f67-5060796b99e4",
  );

  // Project description blocks
  final projectHeaderBlock = BlockEntity(
    uuid: "cognitive-proj-header-001",
    nodeType: "header1",
    content: AttributedText("Cognitive Load in Multi-Modal Interfaces Study"),
  );

  final projectDetailsBlock = BlockEntity(
    uuid: "cognitive-proj-details-002",
    nodeType: "paragraph",
    content: AttributedText(
      "Principal Investigator: Dr. Sarah Chen | Institution: MIT Computer Science & Artificial Intelligence Laboratory | Duration: 18 months | Budget: \$250,000 (NSF Grant #CHI-2024-1847)",
    ),
  );

  final researchQuestionHeaderBlock = BlockEntity(
    uuid: "cognitive-proj-question-003",
    nodeType: "header2",
    content: AttributedText("Research Question"),
  );

  final researchQuestionBlock = BlockEntity(
    uuid: "cognitive-proj-question-004",
    nodeType: "paragraph",
    content: AttributedText(
      "How does cognitive load theory apply to modern multi-modal interfaces combining voice, gesture, and visual elements? Can we develop predictive models for optimal interface design?",
    ),
  );

  final methodotempoDBSeedLogyHeaderBlock = BlockEntity(
    uuid: "cognitive-proj-methodotempoDBSeedLog.y-005",
    nodeType: "header2",
    content: AttributedText("MethodotempoDBSeedLog.y"),
  );

  final methodotempoDBSeedLogyBlock1 = BlockEntity(
    uuid: "cognitive-proj-methodotempoDBSeedLog.y-006",
    nodeType: "list_item",
    content: AttributedText(
      "Participants: 120 subjects across 3 age groups (18-30, 31-50, 51-70)",
    ),
  );

  final methodotempoDBSeedLogyBlock2 = BlockEntity(
    uuid: "cognitive-proj-methodotempoDBSeedLog.y-007",
    nodeType: "list_item",
    content: AttributedText(
      "Tasks: Complex information retrieval using various interface modalities",
    ),
  );

  final methodotempoDBSeedLogyBlock3 = BlockEntity(
    uuid: "cognitive-proj-methodotempoDBSeedLog.y-008",
    nodeType: "list_item",
    content: AttributedText(
      "Measurements: EEG, eye tracking, task completion time, accuracy",
    ),
  );

  final methodotempoDBSeedLogyBlock4 = BlockEntity(
    uuid: "cognitive-proj-methodotempoDBSeedLog.y-009",
    nodeType: "list_item",
    content: AttributedText(
      "Control Variables: Prior experience, domain expertise, interface familiarity",
    ),
  );

  final outcomesHeaderBlock = BlockEntity(
    uuid: "cognitive-proj-outcomes-010",
    nodeType: "header2",
    content: AttributedText("Expected Outcomes"),
  );

  final outcomesBlock1 = BlockEntity(
    uuid: "cognitive-proj-outcomes-011",
    nodeType: "list_item",
    content: AttributedText(
      "Empirical framework for measuring cognitive load in multi-modal contexts",
    ),
  );

  final outcomesBlock2 = BlockEntity(
    uuid: "cognitive-proj-outcomes-012",
    nodeType: "list_item",
    content: AttributedText(
      "Design guidelines for interface complexity management",
    ),
  );

  final outcomesBlock3 = BlockEntity(
    uuid: "cognitive-proj-outcomes-013",
    nodeType: "list_item",
    content: AttributedText("Publication targets: CHI 2025, Journal of HCI"),
  );

  final outcomesBlock4 = BlockEntity(
    uuid: "cognitive-proj-outcomes-014",
    nodeType: "list_item",
    content: AttributedText(
      "PhD dissertation chapter for two graduate students",
    ),
  );

  final irbHeaderBlock = BlockEntity(
    uuid: "cognitive-proj-irb-015",
    nodeType: "header2",
    content: AttributedText("IRB Status"),
  );

  final irbBlock1 = BlockEntity(
    uuid: "cognitive-proj-irb-016",
    nodeType: "list_item",
    content: AttributedText("Protocol #2024-157 approved October 2024"),
  );

  final irbBlock2 = BlockEntity(
    uuid: "cognitive-proj-irb-017",
    nodeType: "list_item",
    content: AttributedText("Participant recruitment beginning December 2024"),
  );

  final literatureTask = DevtoolsTaskDefaults.withDefaults(
    "research-lit-review-01",
    AttributedText("Complete systematic literature review"),
    status: TaskStatus.workInProgress,
    priority: TaskPriority.H,
    dateDelta: const Duration(days: 14),
  );

  // Literature review task description blocks
  final litReviewHeaderBlock = BlockEntity(
    uuid: "cognitive-lit-header-018",
    nodeType: "header3",
    content: AttributedText(
      "Systematic Literature Review: Cognitive Load Theory & Multi-Modal Interfaces",
    ),
  );

  final litSearchHeaderBlock = BlockEntity(
    uuid: "cognitive-lit-search-019",
    nodeType: "header4",
    content: AttributedText("Search Strategy"),
  );

  final litSearchBlock1 = BlockEntity(
    uuid: "cognitive-lit-search-020",
    nodeType: "list_item",
    content: AttributedText(
      "Databases: ACM Digital Library, IEEE Xplore, PsycINFO, Cognitive Science",
    ),
  );

  final litSearchBlock2 = BlockEntity(
    uuid: "cognitive-lit-search-021",
    nodeType: "list_item",
    content: AttributedText(
      "Keywords: \"cognitive load theory\" AND (\"multi-modal\" OR \"multimodal\") AND (\"interface\" OR \"interaction\")",
    ),
  );

  final litSearchBlock3 = BlockEntity(
    uuid: "cognitive-lit-search-022",
    nodeType: "list_item",
    content: AttributedText("Timeframe: 2015-2024 (past 10 years)"),
  );

  final litSearchBlock4 = BlockEntity(
    uuid: "cognitive-lit-search-023",
    nodeType: "list_item",
    content: AttributedText(
      "Initial hits: 847 papers → 156 relevant → 43 high-quality",
    ),
  );

  final litProgressHeaderBlock = BlockEntity(
    uuid: "cognitive-lit-progress-024",
    nodeType: "header4",
    content: AttributedText("Progress"),
  );

  final litProgressBlock1 = BlockEntity(
    uuid: "cognitive-lit-progress-025",
    nodeType: "list_item",
    content: AttributedText("Phase 1: Database search complete ✓"),
  );

  final litProgressBlock2 = BlockEntity(
    uuid: "cognitive-lit-progress-026",
    nodeType: "list_item",
    content: AttributedText("Phase 2: Abstract screening (67% complete)"),
  );

  final litProgressBlock3 = BlockEntity(
    uuid: "cognitive-lit-progress-027",
    nodeType: "list_item",
    content: AttributedText("Phase 3: Full-text review (pending)"),
  );

  final litProgressBlock4 = BlockEntity(
    uuid: "cognitive-lit-progress-028",
    nodeType: "list_item",
    content: AttributedText(
      "Phase 4: Quality assessment using PRISMA guidelines",
    ),
  );

  final litThemesHeaderBlock = BlockEntity(
    uuid: "cognitive-lit-themes-029",
    nodeType: "header4",
    content: AttributedText("Key Themes Emerging"),
  );

  final litThemesBlock1 = BlockEntity(
    uuid: "cognitive-lit-themes-030",
    nodeType: "list_item",
    content: AttributedText("Modality switching costs in complex tasks"),
  );

  final litThemesBlock2 = BlockEntity(
    uuid: "cognitive-lit-themes-031",
    nodeType: "list_item",
    content: AttributedText(
      "Age-related differences in multi-modal processing",
    ),
  );

  final litThemesBlock3 = BlockEntity(
    uuid: "cognitive-lit-themes-032",
    nodeType: "list_item",
    content: AttributedText("Individual differences in cognitive capacity"),
  );

  final litDeliverableBlock = BlockEntity(
    uuid: "cognitive-lit-deliverable-033",
    nodeType: "paragraph",
    content: AttributedText(
      "Deliverable: 25-page review for grant progress report (due January 15)",
    ),
  );

  final protocolTask = DevtoolsTaskDefaults.withDefaults(
    "research-protocol-01",
    AttributedText("Develop experimental protocol"),
    dateDelta: const Duration(days: 28),
    priority: TaskPriority.H,
  );

  // Protocol task description blocks
  final protocolHeaderBlock = BlockEntity(
    uuid: "cognitive-protocol-header-034",
    nodeType: "header3",
    content: AttributedText("Experimental Protocol Development"),
  );

  final studyDesignHeaderBlock = BlockEntity(
    uuid: "cognitive-protocol-design-035",
    nodeType: "header4",
    content: AttributedText("Study Design"),
  );

  final studyDesignBlock1 = BlockEntity(
    uuid: "cognitive-protocol-design-036",
    nodeType: "list_item",
    content: AttributedText(
      "Between-subjects factorial: 3 (modality) × 3 (age group) × 2 (complexity)",
    ),
  );

  final studyDesignBlock2 = BlockEntity(
    uuid: "cognitive-protocol-design-037",
    nodeType: "list_item",
    content: AttributedText(
      "Sample size calculation: G*Power analysis suggests n=120 for medium effect",
    ),
  );

  final studyDesignBlock3 = BlockEntity(
    uuid: "cognitive-protocol-design-038",
    nodeType: "list_item",
    content: AttributedText(
      "Task scenarios: Information search, data entry, system configuration",
    ),
  );

  final studyDesignBlock4 = BlockEntity(
    uuid: "cognitive-protocol-design-039",
    nodeType: "list_item",
    content: AttributedText(
      "Duration per session: 90 minutes (including setup and debriefing)",
    ),
  );

  final apparatusHeaderBlock = BlockEntity(
    uuid: "cognitive-protocol-apparatus-040",
    nodeType: "header4",
    content: AttributedText("Apparatus Setup"),
  );

  final apparatusBlock1 = BlockEntity(
    uuid: "cognitive-protocol-apparatus-041",
    nodeType: "list_item",
    content: AttributedText("EEG: 32-channel Biosemi ActiveTwo system"),
  );

  final apparatusBlock2 = BlockEntity(
    uuid: "cognitive-protocol-apparatus-042",
    nodeType: "list_item",
    content: AttributedText("Eye tracking: Tobii Pro X3-120 remote tracker"),
  );

  final apparatusBlock3 = BlockEntity(
    uuid: "cognitive-protocol-apparatus-043",
    nodeType: "list_item",
    content: AttributedText(
      "Multi-modal interface: Custom React.js application",
    ),
  );

  final apparatusBlock4 = BlockEntity(
    uuid: "cognitive-protocol-apparatus-044",
    nodeType: "list_item",
    content: AttributedText("Voice recognition: Azure Speech Services API"),
  );

  final apparatusBlock5 = BlockEntity(
    uuid: "cognitive-protocol-apparatus-045",
    nodeType: "list_item",
    content: AttributedText("Gesture detection: Leap Motion Controller"),
  );

  final dependentVarsHeaderBlock = BlockEntity(
    uuid: "cognitive-protocol-vars-046",
    nodeType: "header4",
    content: AttributedText("Dependent Variables"),
  );

  final dependentVarsBlock1 = BlockEntity(
    uuid: "cognitive-protocol-vars-047",
    nodeType: "list_item",
    content: AttributedText(
      "Primary: Task completion time, error rate, subjective workload (NASA-TLX)",
    ),
  );

  final dependentVarsBlock2 = BlockEntity(
    uuid: "cognitive-protocol-vars-048",
    nodeType: "list_item",
    content: AttributedText(
      "Secondary: EEG alpha/theta ratio, fixation patterns, modality preference",
    ),
  );

  final qualityHeaderBlock = BlockEntity(
    uuid: "cognitive-protocol-quality-049",
    nodeType: "header4",
    content: AttributedText("Quality Assurance"),
  );

  final qualityBlock1 = BlockEntity(
    uuid: "cognitive-protocol-quality-050",
    nodeType: "list_item",
    content: AttributedText("Pilot study with 12 participants"),
  );

  final qualityBlock2 = BlockEntity(
    uuid: "cognitive-protocol-quality-051",
    nodeType: "list_item",
    content: AttributedText("Inter-rater reliability for behavioral coding"),
  );

  final qualityBlock3 = BlockEntity(
    uuid: "cognitive-protocol-quality-052",
    nodeType: "list_item",
    content: AttributedText("Equipment calibration protocols"),
  );

  final recruitmentTask = DevtoolsTaskDefaults.withDefaults(
    "research-recruitment-01",
    AttributedText("Participant recruitment campaign"),
    status: TaskStatus.scheduled,
    dateDelta: const Duration(days: 45),
  );

  // Recruitment task description blocks
  final recruitmentHeaderBlock = BlockEntity(
    uuid: "cognitive-recruitment-header-053",
    nodeType: "header3",
    content: AttributedText("Participant Recruitment & Screening"),
  );

  final recruitmentStrategyHeaderBlock = BlockEntity(
    uuid: "cognitive-recruitment-strategy-054",
    nodeType: "header4",
    content: AttributedText("Recruitment Strategy"),
  );

  final recruitmentStrategyBlock1 = BlockEntity(
    uuid: "cognitive-recruitment-strategy-055",
    nodeType: "list_item",
    content: AttributedText(
      "Campus outreach: Student email lists, bulletin boards, class announcements",
    ),
  );

  final recruitmentStrategyBlock2 = BlockEntity(
    uuid: "cognitive-recruitment-strategy-056",
    nodeType: "list_item",
    content: AttributedText(
      "Community partners: Senior centers, professional networks",
    ),
  );

  final recruitmentStrategyBlock3 = BlockEntity(
    uuid: "cognitive-recruitment-strategy-057",
    nodeType: "list_item",
    content: AttributedText(
      "Online: Prolific, social media (IRB-approved posts)",
    ),
  );

  final recruitmentStrategyBlock4 = BlockEntity(
    uuid: "cognitive-recruitment-strategy-058",
    nodeType: "list_item",
    content: AttributedText(
      "Compensation: \$50 per session + travel reimbursement",
    ),
  );

  final inclusionHeaderBlock = BlockEntity(
    uuid: "cognitive-recruitment-inclusion-059",
    nodeType: "header4",
    content: AttributedText("Inclusion Criteria"),
  );

  final inclusionBlock1 = BlockEntity(
    uuid: "cognitive-recruitment-inclusion-060",
    nodeType: "list_item",
    content: AttributedText("Ages 18-70, normal or corrected vision"),
  );

  final inclusionBlock2 = BlockEntity(
    uuid: "cognitive-recruitment-inclusion-061",
    nodeType: "list_item",
    content: AttributedText(
      "English fluency, right-handed (for EEG consistency)",
    ),
  );

  final inclusionBlock3 = BlockEntity(
    uuid: "cognitive-recruitment-inclusion-062",
    nodeType: "list_item",
    content: AttributedText("No history of neurotempoDBSeedLog.ical disorders"),
  );

  final inclusionBlock4 = BlockEntity(
    uuid: "cognitive-recruitment-inclusion-063",
    nodeType: "list_item",
    content: AttributedText("Willing to complete 90-minute session"),
  );

  final screeningHeaderBlock = BlockEntity(
    uuid: "cognitive-recruitment-screening-064",
    nodeType: "header4",
    content: AttributedText("Screening Process"),
  );

  final screeningBlock1 = BlockEntity(
    uuid: "cognitive-recruitment-screening-065",
    nodeType: "list_item",
    content: AttributedText("Online pre-screening questionnaire (10 minutes)"),
  );

  final screeningBlock2 = BlockEntity(
    uuid: "cognitive-recruitment-screening-066",
    nodeType: "list_item",
    content: AttributedText("TechnotempoDBSeedLogy familiarity assessment"),
  );

  final screeningBlock3 = BlockEntity(
    uuid: "cognitive-recruitment-screening-067",
    nodeType: "list_item",
    content: AttributedText("Scheduling coordination with lab availability"),
  );

  final screeningBlock4 = BlockEntity(
    uuid: "cognitive-recruitment-screening-068",
    nodeType: "list_item",
    content: AttributedText("Informed consent review via email"),
  );

  final demographicsHeaderBlock = BlockEntity(
    uuid: "cognitive-recruitment-demographics-069",
    nodeType: "header4",
    content: AttributedText("Target Demographics"),
  );

  final demographicsBlock1 = BlockEntity(
    uuid: "cognitive-recruitment-demographics-070",
    nodeType: "list_item",
    content: AttributedText(
      "Age 18-30: 40 participants (students, young professionals)",
    ),
  );

  final demographicsBlock2 = BlockEntity(
    uuid: "cognitive-recruitment-demographics-071",
    nodeType: "list_item",
    content: AttributedText(
      "Age 31-50: 40 participants (working professionals)",
    ),
  );

  final demographicsBlock3 = BlockEntity(
    uuid: "cognitive-recruitment-demographics-072",
    nodeType: "list_item",
    content: AttributedText(
      "Age 51-70: 40 participants (community volunteers)",
    ),
  );

  final trackingBlock = BlockEntity(
    uuid: "cognitive-recruitment-tracking-073",
    nodeType: "paragraph",
    content: AttributedText(
      "Tracking: REDCap database for participant management",
    ),
  );

  final analysisTask = DevtoolsTaskDefaults.withDefaults(
    "research-analysis-01",
    AttributedText("Statistical analysis plan"),
    dateDelta: const Duration(days: 60),
  );

  // Analysis task description blocks
  final analysisHeaderBlock = BlockEntity(
    uuid: "cognitive-analysis-header-074",
    nodeType: "header3",
    content: AttributedText("Statistical Analysis Plan"),
  );

  final primaryAnalysisHeaderBlock = BlockEntity(
    uuid: "cognitive-analysis-primary-075",
    nodeType: "header4",
    content: AttributedText("Primary Analyses"),
  );

  final primaryAnalysisBlock1 = BlockEntity(
    uuid: "cognitive-analysis-primary-076",
    nodeType: "list_item",
    content: AttributedText(
      "ANOVA: Mixed-effects model with modality, age group, complexity as factors",
    ),
  );

  final primaryAnalysisBlock2 = BlockEntity(
    uuid: "cognitive-analysis-primary-077",
    nodeType: "list_item",
    content: AttributedText(
      "Effect sizes: Cohen's d for practical significance assessment",
    ),
  );

  final primaryAnalysisBlock3 = BlockEntity(
    uuid: "cognitive-analysis-primary-078",
    nodeType: "list_item",
    content: AttributedText(
      "Post-hoc tests: Bonferroni-corrected pairwise comparisons",
    ),
  );

  final primaryAnalysisBlock4 = BlockEntity(
    uuid: "cognitive-analysis-primary-079",
    nodeType: "list_item",
    content: AttributedText(
      "Power analysis: Retrospective assessment of achieved power",
    ),
  );

  final secondaryAnalysisHeaderBlock = BlockEntity(
    uuid: "cognitive-analysis-secondary-080",
    nodeType: "header4",
    content: AttributedText("Secondary Analyses"),
  );

  final secondaryAnalysisBlock1 = BlockEntity(
    uuid: "cognitive-analysis-secondary-081",
    nodeType: "list_item",
    content: AttributedText(
      "EEG frequency analysis: FFT on alpha (8-12 Hz) and theta (4-8 Hz) bands",
    ),
  );

  final secondaryAnalysisBlock2 = BlockEntity(
    uuid: "cognitive-analysis-secondary-082",
    nodeType: "list_item",
    content: AttributedText(
      "Eye tracking metrics: Fixation duration, saccade patterns, AOI analysis",
    ),
  );

  final secondaryAnalysisBlock3 = BlockEntity(
    uuid: "cognitive-analysis-secondary-083",
    nodeType: "list_item",
    content: AttributedText(
      "Individual differences: Correlations with working memory capacity",
    ),
  );

  final secondaryAnalysisBlock4 = BlockEntity(
    uuid: "cognitive-analysis-secondary-084",
    nodeType: "list_item",
    content: AttributedText(
      "Qualitative coding: Thematic analysis of post-task interviews",
    ),
  );

  final softwareHeaderBlock = BlockEntity(
    uuid: "cognitive-analysis-software-085",
    nodeType: "header4",
    content: AttributedText("Software Tools"),
  );

  final softwareBlock1 = BlockEntity(
    uuid: "cognitive-analysis-software-086",
    nodeType: "list_item",
    content: AttributedText("Statistical: R (lme4, ggplot2, effsize packages)"),
  );

  final softwareBlock2 = BlockEntity(
    uuid: "cognitive-analysis-software-087",
    nodeType: "list_item",
    content: AttributedText(
      "EEG: EEGLAB/MATLAB for preprocessing and analysis",
    ),
  );

  final softwareBlock3 = BlockEntity(
    uuid: "cognitive-analysis-software-088",
    nodeType: "list_item",
    content: AttributedText(
      "Eye tracking: Tobii Studio for basic metrics, custom R scripts",
    ),
  );

  final softwareBlock4 = BlockEntity(
    uuid: "cognitive-analysis-software-089",
    nodeType: "list_item",
    content: AttributedText(
      "Qualitative: NVivo for interview transcription and coding",
    ),
  );

  final reproducibilityHeaderBlock = BlockEntity(
    uuid: "cognitive-analysis-repro-090",
    nodeType: "header4",
    content: AttributedText("Reproducibility"),
  );

  final reproducibilityBlock1 = BlockEntity(
    uuid: "cognitive-analysis-repro-091",
    nodeType: "list_item",
    content: AttributedText("All analysis code version-controlled in GitHub"),
  );

  final reproducibilityBlock2 = BlockEntity(
    uuid: "cognitive-analysis-repro-092",
    nodeType: "list_item",
    content: AttributedText("Literate programming approach using R Markdown"),
  );

  final reproducibilityBlock3 = BlockEntity(
    uuid: "cognitive-analysis-repro-093",
    nodeType: "list_item",
    content: AttributedText(
      "Data sharing plan compliant with IRB requirements",
    ),
  );

  // Helper methods for additional tasks
  TaskEntity buildProjectScopeTask() {
    return DevtoolsTaskDefaults.withDefaults(
      "research-scope-01",
      AttributedText("Define project scope and objectives"),
      status: TaskStatus.complete,
    );
  }

  TaskEntity buildProjectTimelineTask() {
    return DevtoolsTaskDefaults.withDefaults(
      "research-timeline-01",
      AttributedText("Create project timeline and milestones"),
      status: TaskStatus.complete,
    );
  }

  @override
  void seed(SeedDataService s) {
    tempoDBSeedLog.finer(
      '  <seeding project="${project.title.toPlainText()}">',
    );
    final projectID = s.projectRepo.put(project);

    // Seed project description blocks
    final projectBlockIDs = s.blockRepo.putMany([
      projectHeaderBlock,
      projectDetailsBlock,
      researchQuestionHeaderBlock,
      researchQuestionBlock,
      methodotempoDBSeedLogyHeaderBlock,
      methodotempoDBSeedLogyBlock1,
      methodotempoDBSeedLogyBlock2,
      methodotempoDBSeedLogyBlock3,
      methodotempoDBSeedLogyBlock4,
      outcomesHeaderBlock,
      outcomesBlock1,
      outcomesBlock2,
      outcomesBlock3,
      outcomesBlock4,
      irbHeaderBlock,
      irbBlock1,
      irbBlock2,
    ]);

    // Seed task description blocks
    final litReviewBlockIDs = s.blockRepo.putMany([
      litReviewHeaderBlock,
      litSearchHeaderBlock,
      litSearchBlock1,
      litSearchBlock2,
      litSearchBlock3,
      litSearchBlock4,
      litProgressHeaderBlock,
      litProgressBlock1,
      litProgressBlock2,
      litProgressBlock3,
      litProgressBlock4,
      litThemesHeaderBlock,
      litThemesBlock1,
      litThemesBlock2,
      litThemesBlock3,
      litDeliverableBlock,
    ]);

    final protocolBlockIDs = s.blockRepo.putMany([
      protocolHeaderBlock,
      studyDesignHeaderBlock,
      studyDesignBlock1,
      studyDesignBlock2,
      studyDesignBlock3,
      studyDesignBlock4,
      apparatusHeaderBlock,
      apparatusBlock1,
      apparatusBlock2,
      apparatusBlock3,
      apparatusBlock4,
      apparatusBlock5,
      dependentVarsHeaderBlock,
      dependentVarsBlock1,
      dependentVarsBlock2,
      qualityHeaderBlock,
      qualityBlock1,
      qualityBlock2,
      qualityBlock3,
    ]);

    final recruitmentBlockIDs = s.blockRepo.putMany([
      recruitmentHeaderBlock,
      recruitmentStrategyHeaderBlock,
      recruitmentStrategyBlock1,
      recruitmentStrategyBlock2,
      recruitmentStrategyBlock3,
      recruitmentStrategyBlock4,
      inclusionHeaderBlock,
      inclusionBlock1,
      inclusionBlock2,
      inclusionBlock3,
      inclusionBlock4,
      screeningHeaderBlock,
      screeningBlock1,
      screeningBlock2,
      screeningBlock3,
      screeningBlock4,
      demographicsHeaderBlock,
      demographicsBlock1,
      demographicsBlock2,
      demographicsBlock3,
      trackingBlock,
    ]);

    final analysisBlockIDs = s.blockRepo.putMany([
      analysisHeaderBlock,
      primaryAnalysisHeaderBlock,
      primaryAnalysisBlock1,
      primaryAnalysisBlock2,
      primaryAnalysisBlock3,
      primaryAnalysisBlock4,
      secondaryAnalysisHeaderBlock,
      secondaryAnalysisBlock1,
      secondaryAnalysisBlock2,
      secondaryAnalysisBlock3,
      secondaryAnalysisBlock4,
      softwareHeaderBlock,
      softwareBlock1,
      softwareBlock2,
      softwareBlock3,
      softwareBlock4,
      reproducibilityHeaderBlock,
      reproducibilityBlock1,
      reproducibilityBlock2,
      reproducibilityBlock3,
    ]);

    final scopeTask = buildProjectScopeTask();
    final timelineTask = buildProjectTimelineTask();

    final [
      scopeID,
      timelineID,
      literatureID,
      protocolID,
      recruitmentID,
      analysisID,
    ] = s.taskRepo.putMany([
      scopeTask,
      timelineTask,
      literatureTask,
      protocolTask,
      recruitmentTask,
      analysisTask,
    ]);

    // Set project relationships for all tasks
    final savedProject = s.projectRepo.get(projectID)!;
    tempoDBSeedLog.fine(
      'Setting project relationships for ${savedProject.title.toPlainText()}',
    );
    for (final taskID in [
      scopeID,
      timelineID,
      literatureID,
      protocolID,
      recruitmentID,
      analysisID,
    ]) {
      final task = s.taskRepo.get(taskID)!;
      task.project.target = savedProject;
      s.taskRepo.put(task);
      tempoDBSeedLog.fine(
        'Set project for task ${task.uuid}: ${task.title.toPlainText()} -> ${task.project.target?.title.toPlainText()}',
      );
    }

    // Add blocks to tasks with descriptions
    final savedLiteratureTask = s.taskRepo.get(literatureID)!;
    final savedLiteratureTaskThingEntity = savedLiteratureTask.thing.target!;
    savedLiteratureTaskThingEntity.children.addAll(
      s.blockRepo.getMany(litReviewBlockIDs).map((b) => b!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(savedLiteratureTaskThingEntity);
    s.taskRepo.put(savedLiteratureTask);

    final savedProtocolTask = s.taskRepo.get(protocolID)!;
    final savedProtocolTaskThingEntity = savedProtocolTask.thing.target!;
    savedProtocolTaskThingEntity.children.addAll(
      s.blockRepo.getMany(protocolBlockIDs).map((b) => b!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(savedProtocolTaskThingEntity);
    s.taskRepo.put(savedProtocolTask);

    final savedRecruitmentTask = s.taskRepo.get(recruitmentID)!;
    final savedRecruitmentTaskThingEntity = savedRecruitmentTask.thing.target!;
    savedRecruitmentTaskThingEntity.children.addAll(
      s.blockRepo.getMany(recruitmentBlockIDs).map((b) => b!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(savedRecruitmentTaskThingEntity);
    s.taskRepo.put(savedRecruitmentTask);

    final savedAnalysisTask = s.taskRepo.get(analysisID)!;
    final savedAnalysisTaskThingEntity = savedAnalysisTask.thing.target!;
    savedAnalysisTaskThingEntity.children.addAll(
      s.blockRepo.getMany(analysisBlockIDs).map((b) => b!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(savedAnalysisTaskThingEntity);
    s.taskRepo.put(savedAnalysisTask);

    final savedProjectThingEntity = savedProject.thing.target!;
    savedProjectThingEntity.children.addAll(
      s.blockRepo.getMany(projectBlockIDs).map((b) => b!.thing.target!),
    );
    savedProjectThingEntity.children.addAll(
      s.taskRepo
          .getMany([
            scopeID,
            timelineID,
            literatureID,
            protocolID,
            recruitmentID,
            analysisID,
          ])
          .map((t) => t!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.projectRepo.store.box<ThingEntity>().put(savedProjectThingEntity);
    s.projectRepo.put(savedProject);
  }
}
