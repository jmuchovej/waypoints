import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project.entity.dart';
import '../../../things/task/task.entity.dart';
import '../../../things/block/block.entity.dart';
import '../../../things/thing/thing.entity.dart';
import '../seed.dart';
import '_seed.dart';

class APIv2MigrationProject extends DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("API v2 Migration"),
    uuid: "b837b01d-9a4f-43c5-ac95-3ff454be411d",
  );

  // Project description blocks
  final projectOverviewBlock = BlockEntity(
    uuid: "api-overview-block-001",
    nodeType: "header1",
    content: AttributedText("API v2 Migration Project"),
  );

  final projectDescBlock = BlockEntity(
    uuid: "api-overview-block-002",
    nodeType: "paragraph",
    content: AttributedText(
      "Migration from our legacy v1 API to the new GraphQL-based v2 architecture. This involves updating all endpoints, client libraries, and ensuring backward compatibility during the transition period.",
    ),
  );

  final objectivesHeaderBlock = BlockEntity(
    uuid: "api-overview-block-003",
    nodeType: "header2",
    content: AttributedText("Key Objectives"),
  );

  final objectivesListBlock = BlockEntity(
    uuid: "api-overview-block-004",
    nodeType: "list_item",
    content: AttributedText(
      "Migrate 47 v1 REST endpoints to GraphQL\nUpdate client SDKs for JavaScript, Python, iOS, and Android\nMaintain 6-month backward compatibility\nImprove API response times by 40%",
    ),
  );

  final successHeaderBlock = BlockEntity(
    uuid: "api-overview-block-005",
    nodeType: "header2",
    content: AttributedText("Success Criteria"),
  );

  final successListBlock = BlockEntity(
    uuid: "api-overview-block-006",
    nodeType: "list_item",
    content: AttributedText(
      "All critical endpoints migrated and tested\nZero breaking changes for existing clients\nPerformance benchmarks met\nDocumentation updated with migration guides",
    ),
  );

  final apiEndpointTask = DevtoolsTaskDefaults.withDefaults(
    "cf6459d7-e27b-446c-bd9e-6d0912f32835",
    AttributedText("Document current API endpoints"),
    status: TaskStatus.complete,
    dateDelta: const Duration(days: 7),
  );

  // API endpoint task description blocks
  final apiEndpointDescBlock = BlockEntity(
    uuid: "api-endpoint-desc-001",
    nodeType: "paragraph",
    content: AttributedText("Comprehensive audit of existing v1 API endpoints"),
  );

  final apiEndpointDetailsBlock = BlockEntity(
    uuid: "api-endpoint-desc-002",
    nodeType: "paragraph",
    content: AttributedText(
      "Create detailed documentation of all current REST endpoints including HTTP methods, URL patterns, request/response schemas, authentication requirements, rate limiting rules, and usage metrics from past 6 months.",
    ),
  );

  final apiEndpointImportanceBlock = BlockEntity(
    uuid: "api-endpoint-desc-003",
    nodeType: "paragraph",
    content: AttributedText(
      "This foundation is critical for planning the GraphQL migration strategy.",
    ),
  );

  final listV1EndpointsTask = DevtoolsTaskDefaults.withDefaults(
    "73634e1b-121c-4ae0-89e6-f05e0f45aba0",
    AttributedText("List all v1 endpoints with parameters"),
    status: TaskStatus.complete,
  );

  // V1 endpoints task description blocks
  final v1EndpointsDescBlock = BlockEntity(
    uuid: "api-v1-desc-007",
    nodeType: "paragraph",
    content: AttributedText("Detailed inventory of REST endpoints:"),
  );

  final v1EndpointsListBlock1 = BlockEntity(
    uuid: "api-v1-list-008",
    nodeType: "list_item",
    content: AttributedText(
      "/api/v1/users - User management (GET, POST, PUT, DELETE)",
    ),
  );

  final v1EndpointsListBlock2 = BlockEntity(
    uuid: "api-v1-list-009",
    nodeType: "list_item",
    content: AttributedText(
      "/api/v1/organizations - Org management with nested resources",
    ),
  );

  final v1EndpointsListBlock3 = BlockEntity(
    uuid: "api-v1-list-010",
    nodeType: "list_item",
    content: AttributedText(
      "/api/v1/projects/{id}/tasks - Project task aggregations",
    ),
  );

  final v1EndpointsListBlock4 = BlockEntity(
    uuid: "api-v1-list-011",
    nodeType: "list_item",
    content: AttributedText(
      "/api/v1/analytics/* - 23 analytics endpoints with complex filters",
    ),
  );

  final v1EndpointsListBlock5 = BlockEntity(
    uuid: "api-v1-list-012",
    nodeType: "list_item",
    content: AttributedText(
      "/api/v1/integrations/* - Webhook and OAuth endpoints",
    ),
  );

  final v1EndpointsStatusBlock = BlockEntity(
    uuid: "api-v1-status-013",
    nodeType: "paragraph",
    content: AttributedText(
      "Status: Complete inventory of 47 endpoints documented in Confluence.",
    ),
  );

  final listDeprecatedEndpointsTask = DevtoolsTaskDefaults.withDefaults(
    "07adc303-ca45-4673-951f-28f85093d8c2",
    AttributedText("Identify deprecated endpoints"),
    status: TaskStatus.complete,
  );

  // Deprecated endpoints task description blocks
  final deprecatedHeaderBlock = BlockEntity(
    uuid: "api-deprecated-header-014",
    nodeType: "paragraph",
    content: AttributedText("Deprecated endpoints flagged for removal:"),
  );

  final deprecatedListBlock1 = BlockEntity(
    uuid: "api-deprecated-list-015",
    nodeType: "list_item",
    content: AttributedText(
      "/api/v1/legacy/reports - Replaced by new analytics",
    ),
  );

  final deprecatedListBlock2 = BlockEntity(
    uuid: "api-deprecated-list-016",
    nodeType: "list_item",
    content: AttributedText("/api/v1/old/webhooks - Security vulnerabilities"),
  );

  final deprecatedListBlock3 = BlockEntity(
    uuid: "api-deprecated-list-017",
    nodeType: "list_item",
    content: AttributedText(
      "/api/v1/temp/* - Experimental endpoints from 2022",
    ),
  );

  final deprecatedAnalysisBlock = BlockEntity(
    uuid: "api-deprecated-analysis-018",
    nodeType: "paragraph",
    content: AttributedText(
      "Usage Analysis: 3 endpoints still have active traffic (<0.1% of total), deprecation notices sent to affected clients, final sunset date Q2 2025.",
    ),
  );

  final mapEndpointUpgradesTask = DevtoolsTaskDefaults.withDefaults(
    "230795af-02c6-4997-9395-300a2fc7f2ab",
    AttributedText("Map v1 to v2 endpoint changes"),
    status: TaskStatus.complete,
  );

  // Migration mapping task description blocks
  final migrationHeaderBlock = BlockEntity(
    uuid: "api-migration-header-019",
    nodeType: "header3",
    content: AttributedText("REST to GraphQL Migration Mapping"),
  );

  final migrationConsolidationBlock = BlockEntity(
    uuid: "api-migration-consolidation-020",
    nodeType: "paragraph",
    content: AttributedText(
      "Query Consolidation: Multiple GET endpoints become single GraphQL queries. Example: /api/v1/users/{id} + /api/v1/users/{id}/profile → user(id: ID) { profile }",
    ),
  );

  final migrationPatternsBlock = BlockEntity(
    uuid: "api-migration-patterns-021",
    nodeType: "paragraph",
    content: AttributedText(
      "Mutation Patterns: CRUD operations become GraphQL mutations with consistent naming, batch operations now possible with single request.",
    ),
  );

  final migrationBreakingBlock = BlockEntity(
    uuid: "api-migration-breaking-022",
    nodeType: "paragraph",
    content: AttributedText(
      "Breaking Changes: Date format standardized to ISO 8601, error responses restructured for GraphQL spec, pagination uses cursor-based approach.",
    ),
  );

  final migrationGuideBlock = BlockEntity(
    uuid: "api-migration-guide-023",
    nodeType: "paragraph",
    content: AttributedText(
      "Migration guide drafted with before/after examples.",
    ),
  );

  final updateClientLibrariesTask = DevtoolsTaskDefaults.withDefaults(
    "6102f6c9-5d54-4a8c-8043-1eb87e7a1a06",
    AttributedText("Update client libraries"),
    dateDelta: const Duration(days: 21),
    priority: TaskPriority.H,
  );

  // Update client libraries task description blocks
  final clientLibsHeaderBlock = BlockEntity(
    uuid: "api-clientlibs-header-024",
    nodeType: "header3",
    content: AttributedText("Coordinated SDK Migration Initiative"),
  );

  final clientLibsOverviewBlock = BlockEntity(
    uuid: "api-clientlibs-overview-025",
    nodeType: "paragraph",
    content: AttributedText(
      "Update all official client libraries to support GraphQL v2 API while maintaining backward compatibility with v1.",
    ),
  );

  final clientLibsScopeHeaderBlock = BlockEntity(
    uuid: "api-clientlibs-scope-026",
    nodeType: "header4",
    content: AttributedText("Scope"),
  );

  final clientLibsScopeBlock1 = BlockEntity(
    uuid: "api-clientlibs-scope-027",
    nodeType: "list_item",
    content: AttributedText("JavaScript SDK (npm: @company/api-client)"),
  );

  final clientLibsScopeBlock2 = BlockEntity(
    uuid: "api-clientlibs-scope-028",
    nodeType: "list_item",
    content: AttributedText("Python SDK (PyPI: company-api)"),
  );

  final clientLibsScopeBlock3 = BlockEntity(
    uuid: "api-clientlibs-scope-029",
    nodeType: "list_item",
    content: AttributedText("iOS SDK (Swift Package Manager)"),
  );

  final clientLibsScopeBlock4 = BlockEntity(
    uuid: "api-clientlibs-scope-030",
    nodeType: "list_item",
    content: AttributedText("Android SDK (Maven Central)"),
  );

  final clientLibsReqsHeaderBlock = BlockEntity(
    uuid: "api-clientlibs-reqs-031",
    nodeType: "header4",
    content: AttributedText("Requirements"),
  );

  final clientLibsReqsBlock1 = BlockEntity(
    uuid: "api-clientlibs-reqs-032",
    nodeType: "list_item",
    content: AttributedText("Dual API support during transition period"),
  );

  final clientLibsReqsBlock2 = BlockEntity(
    uuid: "api-clientlibs-reqs-033",
    nodeType: "list_item",
    content: AttributedText("Auto-generated code from GraphQL schema"),
  );

  final clientLibsReqsBlock3 = BlockEntity(
    uuid: "api-clientlibs-reqs-034",
    nodeType: "list_item",
    content: AttributedText("Comprehensive test coverage"),
  );

  final clientLibsReqsBlock4 = BlockEntity(
    uuid: "api-clientlibs-reqs-035",
    nodeType: "list_item",
    content: AttributedText("Migration documentation"),
  );

  final clientLibsTimelineBlock = BlockEntity(
    uuid: "api-clientlibs-timeline-036",
    nodeType: "paragraph",
    content: AttributedText("Timeline: 3 weeks with staggered releases"),
  );

  final jsSDKTask = DevtoolsTaskDefaults.withDefaults(
    "26f9cd7b-b990-4624-9def-3e9ff4a8cae8",
    AttributedText("Update JavaScript SDK"),
  );

  // JavaScript SDK task description blocks
  final jsSDKHeaderBlock = BlockEntity(
    uuid: "api-jssdk-header-037",
    nodeType: "header3",
    content: AttributedText("@company/api-client v3.0.0"),
  );

  final jsSDKChangesHeaderBlock = BlockEntity(
    uuid: "api-jssdk-changes-038",
    nodeType: "header4",
    content: AttributedText("Changes"),
  );

  final jsSDKChangesBlock1 = BlockEntity(
    uuid: "api-jssdk-changes-039",
    nodeType: "list_item",
    content: AttributedText(
      "GraphQL client using Apollo Client under the hood",
    ),
  );

  final jsSDKChangesBlock2 = BlockEntity(
    uuid: "api-jssdk-changes-040",
    nodeType: "list_item",
    content: AttributedText("TypeScript-first with auto-generated types"),
  );

  final jsSDKChangesBlock3 = BlockEntity(
    uuid: "api-jssdk-changes-041",
    nodeType: "list_item",
    content: AttributedText("React hooks for common queries"),
  );

  final jsSDKChangesBlock4 = BlockEntity(
    uuid: "api-jssdk-changes-042",
    nodeType: "list_item",
    content: AttributedText(
      "Backwards compatibility adapter for v1 REST calls",
    ),
  );

  final jsSDKImplHeaderBlock = BlockEntity(
    uuid: "api-jssdk-impl-043",
    nodeType: "header4",
    content: AttributedText("Implementation"),
  );

  final jsSDKImplBlock1 = BlockEntity(
    uuid: "api-jssdk-impl-044",
    nodeType: "list_item",
    content: AttributedText("Use GraphQL Code Generator for type safety"),
  );

  final jsSDKImplBlock2 = BlockEntity(
    uuid: "api-jssdk-impl-045",
    nodeType: "list_item",
    content: AttributedText("Bundle size reduction: 45KB → 32KB (gzipped)"),
  );

  final jsSDKImplBlock3 = BlockEntity(
    uuid: "api-jssdk-impl-046",
    nodeType: "list_item",
    content: AttributedText("Breaking changes documented in MIGRATION.md"),
  );

  final jsSDKTestingHeaderBlock = BlockEntity(
    uuid: "api-jssdk-testing-047",
    nodeType: "header4",
    content: AttributedText("Testing"),
  );

  final jsSDKTestingBlock1 = BlockEntity(
    uuid: "api-jssdk-testing-048",
    nodeType: "list_item",
    content: AttributedText("Unit tests for all generated operations"),
  );

  final jsSDKTestingBlock2 = BlockEntity(
    uuid: "api-jssdk-testing-049",
    nodeType: "list_item",
    content: AttributedText("E2E tests against staging GraphQL endpoint"),
  );

  final jsSDKTestingBlock3 = BlockEntity(
    uuid: "api-jssdk-testing-050",
    nodeType: "list_item",
    content: AttributedText("Performance benchmarks vs v2.x"),
  );

  final pySDKTask = DevtoolsTaskDefaults.withDefaults(
    "c0e664f6-fe5f-4f95-9e01-63bbb3781705",
    AttributedText("Update Python SDK"),
  );

  // Python SDK task description blocks
  final pySDKHeaderBlock = BlockEntity(
    uuid: "api-pysdk-header-051",
    nodeType: "header3",
    content: AttributedText("company-api v4.0.0 (Python)"),
  );

  final pySDKFeaturesHeaderBlock = BlockEntity(
    uuid: "api-pysdk-features-052",
    nodeType: "header4",
    content: AttributedText("Features"),
  );

  final pySDKFeaturesBlock1 = BlockEntity(
    uuid: "api-pysdk-features-053",
    nodeType: "list_item",
    content: AttributedText("Native async/await support with aiohttp"),
  );

  final pySDKFeaturesBlock2 = BlockEntity(
    uuid: "api-pysdk-features-054",
    nodeType: "list_item",
    content: AttributedText("Pydantic models for request/response validation"),
  );

  final pySDKFeaturesBlock3 = BlockEntity(
    uuid: "api-pysdk-features-055",
    nodeType: "list_item",
    content: AttributedText("GraphQL introspection for schema validation"),
  );

  final pySDKFeaturesBlock4 = BlockEntity(
    uuid: "api-pysdk-features-056",
    nodeType: "list_item",
    content: AttributedText("CLI tool for common operations"),
  );

  final pySDKArchHeaderBlock = BlockEntity(
    uuid: "api-pysdk-arch-057",
    nodeType: "header4",
    content: AttributedText("Architecture"),
  );

  final pySDKArchBlock1 = BlockEntity(
    uuid: "api-pysdk-arch-058",
    nodeType: "list_item",
    content: AttributedText("Abstract base client for both REST and GraphQL"),
  );

  final pySDKArchBlock2 = BlockEntity(
    uuid: "api-pysdk-arch-059",
    nodeType: "list_item",
    content: AttributedText("Automatic retries with exponential backoff"),
  );

  final pySDKArchBlock3 = BlockEntity(
    uuid: "api-pysdk-arch-060",
    nodeType: "list_item",
    content: AttributedText("Built-in rate limiting and caching"),
  );

  final pySDKArchBlock4 = BlockEntity(
    uuid: "api-pysdk-arch-061",
    nodeType: "list_item",
    content: AttributedText("Plugin system for custom middleware"),
  );

  final pySDKCompatHeaderBlock = BlockEntity(
    uuid: "api-pysdk-compat-062",
    nodeType: "header4",
    content: AttributedText("Compatibility"),
  );

  final pySDKCompatBlock1 = BlockEntity(
    uuid: "api-pysdk-compat-063",
    nodeType: "list_item",
    content: AttributedText("Python 3.8+ support"),
  );

  final pySDKCompatBlock2 = BlockEntity(
    uuid: "api-pysdk-compat-064",
    nodeType: "list_item",
    content: AttributedText("Type hints throughout"),
  );

  final pySDKCompatBlock3 = BlockEntity(
    uuid: "api-pysdk-compat-065",
    nodeType: "list_item",
    content: AttributedText("Dataclasses for complex objects"),
  );

  final mobileSDKsTask = DevtoolsTaskDefaults.withDefaults(
    "7454f602-0d36-4740-8380-3b8bed66e758",
    AttributedText("Update mobile SDKs (iOS/Android)"),
  );

  // Mobile SDKs task description blocks
  final mobileSDKsHeaderBlock = BlockEntity(
    uuid: "api-mobilesdks-header-066",
    nodeType: "header3",
    content: AttributedText("Mobile SDK Modernization"),
  );

  final iOSSDKHeaderBlock = BlockEntity(
    uuid: "api-mobilesdks-ios-067",
    nodeType: "header4",
    content: AttributedText("iOS SDK (Swift)"),
  );

  final iOSSDKBlock1 = BlockEntity(
    uuid: "api-mobilesdks-ios-068",
    nodeType: "list_item",
    content: AttributedText("Apollo iOS for GraphQL operations"),
  );

  final iOSSDKBlock2 = BlockEntity(
    uuid: "api-mobilesdks-ios-069",
    nodeType: "list_item",
    content: AttributedText("Combine integration for reactive streams"),
  );

  final iOSSDKBlock3 = BlockEntity(
    uuid: "api-mobilesdks-ios-070",
    nodeType: "list_item",
    content: AttributedText("SwiftUI-friendly async/await patterns"),
  );

  final iOSSDKBlock4 = BlockEntity(
    uuid: "api-mobilesdks-ios-071",
    nodeType: "list_item",
    content: AttributedText("Keychain integration for secure token storage"),
  );

  final androidSDKHeaderBlock = BlockEntity(
    uuid: "api-mobilesdks-android-072",
    nodeType: "header4",
    content: AttributedText("Android SDK (Kotlin)"),
  );

  final androidSDKBlock1 = BlockEntity(
    uuid: "api-mobilesdks-android-073",
    nodeType: "list_item",
    content: AttributedText("Apollo Android with coroutines"),
  );

  final androidSDKBlock2 = BlockEntity(
    uuid: "api-mobilesdks-android-074",
    nodeType: "list_item",
    content: AttributedText("Retrofit fallback for REST compatibility"),
  );

  final androidSDKBlock3 = BlockEntity(
    uuid: "api-mobilesdks-android-075",
    nodeType: "list_item",
    content: AttributedText("Jetpack Compose state management"),
  );

  final androidSDKBlock4 = BlockEntity(
    uuid: "api-mobilesdks-android-076",
    nodeType: "list_item",
    content: AttributedText("WorkManager for background sync"),
  );

  final sharedFeaturesHeaderBlock = BlockEntity(
    uuid: "api-mobilesdks-shared-077",
    nodeType: "header4",
    content: AttributedText("Shared Features"),
  );

  final sharedFeaturesBlock1 = BlockEntity(
    uuid: "api-mobilesdks-shared-078",
    nodeType: "list_item",
    content: AttributedText("Offline-first architecture with local caching"),
  );

  final sharedFeaturesBlock2 = BlockEntity(
    uuid: "api-mobilesdks-shared-079",
    nodeType: "list_item",
    content: AttributedText("Auto-retry with connectivity awareness"),
  );

  final sharedFeaturesBlock3 = BlockEntity(
    uuid: "api-mobilesdks-shared-080",
    nodeType: "list_item",
    content: AttributedText("Built-in analytics and error reporting"),
  );

  final sharedFeaturesBlock4 = BlockEntity(
    uuid: "api-mobilesdks-shared-081",
    nodeType: "list_item",
    content: AttributedText("Consistent API surface across platforms"),
  );

  @override
  void seed(SeedDataService s) {
    tempoDBSeedLog.finer(
      '  <seeding project="${project.title.toPlainText()}">',
    );
    final projectID = s.projectRepo.put(project);

    // Seed project description blocks
    final projectBlockIDs = s.blockRepo.putMany([
      projectOverviewBlock,
      projectDescBlock,
      objectivesHeaderBlock,
      objectivesListBlock,
      successHeaderBlock,
      successListBlock,
    ]);

    // Seed task description blocks
    final apiEndpointBlockIDs = s.blockRepo.putMany([
      apiEndpointDescBlock,
      apiEndpointDetailsBlock,
      apiEndpointImportanceBlock,
    ]);

    final v1EndpointsBlockIDs = s.blockRepo.putMany([
      v1EndpointsDescBlock,
      v1EndpointsListBlock1,
      v1EndpointsListBlock2,
      v1EndpointsListBlock3,
      v1EndpointsListBlock4,
      v1EndpointsListBlock5,
      v1EndpointsStatusBlock,
    ]);

    final deprecatedBlockIDs = s.blockRepo.putMany([
      deprecatedHeaderBlock,
      deprecatedListBlock1,
      deprecatedListBlock2,
      deprecatedListBlock3,
      deprecatedAnalysisBlock,
    ]);

    final migrationBlockIDs = s.blockRepo.putMany([
      migrationHeaderBlock,
      migrationConsolidationBlock,
      migrationPatternsBlock,
      migrationBreakingBlock,
      migrationGuideBlock,
    ]);

    final clientLibsBlockIDs = s.blockRepo.putMany([
      clientLibsHeaderBlock,
      clientLibsOverviewBlock,
      clientLibsScopeHeaderBlock,
      clientLibsScopeBlock1,
      clientLibsScopeBlock2,
      clientLibsScopeBlock3,
      clientLibsScopeBlock4,
      clientLibsReqsHeaderBlock,
      clientLibsReqsBlock1,
      clientLibsReqsBlock2,
      clientLibsReqsBlock3,
      clientLibsReqsBlock4,
      clientLibsTimelineBlock,
    ]);

    final jsSDKBlockIDs = s.blockRepo.putMany([
      jsSDKHeaderBlock,
      jsSDKChangesHeaderBlock,
      jsSDKChangesBlock1,
      jsSDKChangesBlock2,
      jsSDKChangesBlock3,
      jsSDKChangesBlock4,
      jsSDKImplHeaderBlock,
      jsSDKImplBlock1,
      jsSDKImplBlock2,
      jsSDKImplBlock3,
      jsSDKTestingHeaderBlock,
      jsSDKTestingBlock1,
      jsSDKTestingBlock2,
      jsSDKTestingBlock3,
    ]);

    final pySDKBlockIDs = s.blockRepo.putMany([
      pySDKHeaderBlock,
      pySDKFeaturesHeaderBlock,
      pySDKFeaturesBlock1,
      pySDKFeaturesBlock2,
      pySDKFeaturesBlock3,
      pySDKFeaturesBlock4,
      pySDKArchHeaderBlock,
      pySDKArchBlock1,
      pySDKArchBlock2,
      pySDKArchBlock3,
      pySDKArchBlock4,
      pySDKCompatHeaderBlock,
      pySDKCompatBlock1,
      pySDKCompatBlock2,
      pySDKCompatBlock3,
    ]);

    final mobileSDKsBlockIDs = s.blockRepo.putMany([
      mobileSDKsHeaderBlock,
      iOSSDKHeaderBlock,
      iOSSDKBlock1,
      iOSSDKBlock2,
      iOSSDKBlock3,
      iOSSDKBlock4,
      androidSDKHeaderBlock,
      androidSDKBlock1,
      androidSDKBlock2,
      androidSDKBlock3,
      androidSDKBlock4,
      sharedFeaturesHeaderBlock,
      sharedFeaturesBlock1,
      sharedFeaturesBlock2,
      sharedFeaturesBlock3,
      sharedFeaturesBlock4,
    ]);

    // API endpoint task and subtasks
    final [
      apiEndpointID,
      listV1EndpointsID,
      listDeprecatedEndpointsID,
      mapEndpointUpgradesID,
    ] = s.taskRepo.putMany([
      apiEndpointTask,
      listV1EndpointsTask,
      listDeprecatedEndpointsTask,
      mapEndpointUpgradesTask,
    ]);

    // Set project relationships for all tasks
    final savedProject = s.projectRepo.get(projectID)!;
    tempoDBSeedLog.fine(
      'Setting project relationships for ${savedProject.title.toPlainText()}',
    );
    for (final taskID in [
      apiEndpointID,
      listV1EndpointsID,
      listDeprecatedEndpointsID,
      mapEndpointUpgradesID,
    ]) {
      final task = s.taskRepo.get(taskID)!;
      task.project.target = savedProject;
      s.taskRepo.put(task);
      tempoDBSeedLog.fine(
        'Set project for task ${task.uuid}: ${task.title.toPlainText()} -> ${task.project.target?.title.toPlainText()}',
      );
    }

    // Add blocks as children to specific tasks
    final savedApiEndpointTask = s.taskRepo.get(apiEndpointID)!;
    final savedApiEndpointTaskThingEntity = savedApiEndpointTask.thing.target!;
    savedApiEndpointTaskThingEntity.children.addAll(
      s.blockRepo.getMany(apiEndpointBlockIDs).map((b) => b!.thing.target!),
    );
    savedApiEndpointTaskThingEntity.children.addAll(
      s.taskRepo
          .getMany([
            listV1EndpointsID,
            listDeprecatedEndpointsID,
            mapEndpointUpgradesID,
          ])
          .map((t) => t!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(savedApiEndpointTaskThingEntity);
    s.taskRepo.put(savedApiEndpointTask);

    // Add blocks to V1 endpoints task
    final savedV1EndpointsTask = s.taskRepo.get(listV1EndpointsID)!;
    final savedV1EndpointsTaskThingEntity = savedV1EndpointsTask.thing.target!;
    savedV1EndpointsTaskThingEntity.children.addAll(
      s.blockRepo.getMany(v1EndpointsBlockIDs).map((b) => b!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(savedV1EndpointsTaskThingEntity);
    s.taskRepo.put(savedV1EndpointsTask);

    // Add blocks to deprecated endpoints task
    final savedDeprecatedTask = s.taskRepo.get(listDeprecatedEndpointsID)!;
    final savedDeprecatedTaskThingEntity = savedDeprecatedTask.thing.target!;
    savedDeprecatedTaskThingEntity.children.addAll(
      s.blockRepo.getMany(deprecatedBlockIDs).map((b) => b!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(savedDeprecatedTaskThingEntity);
    s.taskRepo.put(savedDeprecatedTask);

    // Add blocks to migration mapping task
    final savedMigrationTask = s.taskRepo.get(mapEndpointUpgradesID)!;
    final savedMigrationTaskThingEntity = savedMigrationTask.thing.target!;
    savedMigrationTaskThingEntity.children.addAll(
      s.blockRepo.getMany(migrationBlockIDs).map((b) => b!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(savedMigrationTaskThingEntity);
    s.taskRepo.put(savedMigrationTask);

    // Update client libraries task and subtasks
    final [updateClientLibrariesID, jsSDKID, pySDKID, mobileSDKsID] = s.taskRepo
        .putMany([
          updateClientLibrariesTask,
          jsSDKTask,
          pySDKTask,
          mobileSDKsTask,
        ]);

    // Set project relationships for client library tasks
    for (final taskID in [
      updateClientLibrariesID,
      jsSDKID,
      pySDKID,
      mobileSDKsID,
    ]) {
      final task = s.taskRepo.get(taskID)!;
      task.project.target = savedProject;
      s.taskRepo.put(task);
    }

    final savedUpdateClientLibrariesTask =
        s.taskRepo.get(updateClientLibrariesID)!;
    final savedUpdateClientLibrariesTaskThingEntity =
        savedUpdateClientLibrariesTask.thing.target!;
    savedUpdateClientLibrariesTaskThingEntity.children.addAll(
      s.blockRepo.getMany(clientLibsBlockIDs).map((b) => b!.thing.target!),
    );
    savedUpdateClientLibrariesTaskThingEntity.children.addAll(
      s.taskRepo
          .getMany([jsSDKID, pySDKID, mobileSDKsID])
          .map((t) => t!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(
      savedUpdateClientLibrariesTaskThingEntity,
    );
    s.taskRepo.put(savedUpdateClientLibrariesTask);

    // Add blocks to SDK subtasks
    final savedJsSDKTask = s.taskRepo.get(jsSDKID)!;
    final savedJsSDKTaskThingEntity = savedJsSDKTask.thing.target!;
    savedJsSDKTaskThingEntity.children.addAll(
      s.blockRepo.getMany(jsSDKBlockIDs).map((b) => b!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(savedJsSDKTaskThingEntity);
    s.taskRepo.put(savedJsSDKTask);

    final savedPySDKTask = s.taskRepo.get(pySDKID)!;
    final savedPySDKTaskThingEntity = savedPySDKTask.thing.target!;
    savedPySDKTaskThingEntity.children.addAll(
      s.blockRepo.getMany(pySDKBlockIDs).map((b) => b!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(savedPySDKTaskThingEntity);
    s.taskRepo.put(savedPySDKTask);

    final savedMobileSDKsTask = s.taskRepo.get(mobileSDKsID)!;
    final savedMobileSDKsTaskThingEntity = savedMobileSDKsTask.thing.target!;
    savedMobileSDKsTaskThingEntity.children.addAll(
      s.blockRepo.getMany(mobileSDKsBlockIDs).map((b) => b!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(savedMobileSDKsTaskThingEntity);
    s.taskRepo.put(savedMobileSDKsTask);

    // Update project with children (project already loaded above)
    final savedProjectThingEntity = savedProject.thing.target!;
    savedProjectThingEntity.children.addAll(
      s.blockRepo.getMany(projectBlockIDs).map((b) => b!.thing.target!),
    );
    savedProjectThingEntity.children.addAll(
      s.taskRepo
          .getMany([apiEndpointID, updateClientLibrariesID])
          .map((t) => t!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.projectRepo.store.box<ThingEntity>().put(savedProjectThingEntity);
    s.projectRepo.put(savedProject);
  }
}
