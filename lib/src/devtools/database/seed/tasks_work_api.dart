import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project.dart';
import '../../../things/task/task_entity.dart';
import '../seed.dart';
import '_seed.dart';

class APIv2MigrationProject implements DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("API v2 Migration"),
    uuid: "b837b01d-9a4f-43c5-ac95-3ff454be411d",
  );

  final apiEndpointTask = DevtoolsTaskDefaults.withDefaults(
    "cf6459d7-e27b-446c-bd9e-6d0912f32835",
    AttributedText("Document current API endpoints"),
    status: TaskStatus.complete,
    dateDelta: const Duration(days: 7),
  );

  final listV1EndpointsTask = DevtoolsTaskDefaults.withDefaults(
    "73634e1b-121c-4ae0-89e6-f05e0f45aba0",
    AttributedText("List all v1 endpoints with parameters"),
    status: TaskStatus.complete,
  );

  final listDeprecatedEndpointsTask = DevtoolsTaskDefaults.withDefaults(
    "07adc303-ca45-4673-951f-28f85093d8c2",
    AttributedText("Identify deprecated endpoints"),
    status: TaskStatus.complete,
  );

  final mapEndpointUpgradesTask = DevtoolsTaskDefaults.withDefaults(
    "230795af-02c6-4997-9395-300a2fc7f2ab",
    AttributedText("Map v1 to v2 endpoint changes"),
    status: TaskStatus.complete,
  );

  final updateClientLibrariesTask = DevtoolsTaskDefaults.withDefaults(
    "6102f6c9-5d54-4a8c-8043-1eb87e7a1a06",
    AttributedText("Update client libraries"),
    dateDelta: const Duration(days: 21),
    priority: TaskPriority.H,
  );

  final jsSDKTask = DevtoolsTaskDefaults.withDefaults(
    "26f9cd7b-b990-4624-9def-3e9ff4a8cae8",
    AttributedText("Update JavaScript SDK"),
  );

  final pySDKTask = DevtoolsTaskDefaults.withDefaults(
    "c0e664f6-fe5f-4f95-9e01-63bbb3781705",
    AttributedText("Update Python SDK"),
  );

  final mobileSDKsTask = DevtoolsTaskDefaults.withDefaults(
    "7454f602-0d36-4740-8380-3b8bed66e758",
    AttributedText("Update mobile SDKs (iOS/Android)"),
  );

  @override
  void seed(SeedDataService s) {
    dbLog.finer('  <seeding project="${project.titlePlainText}">');
    final projectID = s.projectRepo.put(project);

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

    final savedApiEndpointTask = s.taskRepo.get(apiEndpointID)!;
    savedApiEndpointTask.children.addAll(
      s.taskRepo
          .getMany([
            listV1EndpointsID,
            listDeprecatedEndpointsID,
            mapEndpointUpgradesID,
          ])
          .map((t) => t!.asThingEntity()),
    );
    s.taskRepo.put(savedApiEndpointTask);

    // Update client libraries task and subtasks
    final [updateClientLibrariesID, jsSDKID, pySDKID, mobileSDKsID] = s.taskRepo
        .putMany([
          updateClientLibrariesTask,
          jsSDKTask,
          pySDKTask,
          mobileSDKsTask,
        ]);

    final savedUpdateClientLibrariesTask =
        s.taskRepo.get(updateClientLibrariesID)!;
    savedUpdateClientLibrariesTask.children.addAll(
      s.taskRepo
          .getMany([jsSDKID, pySDKID, mobileSDKsID])
          .map((t) => t!.asThingEntity()),
    );
    s.taskRepo.put(savedUpdateClientLibrariesTask);

    final savedProject = s.projectRepo.get(projectID)!;
    savedProject.children.addAll(
      s.taskRepo
          .getMany([apiEndpointID, updateClientLibrariesID])
          .map((t) => t!.asThingEntity()),
    );
    s.projectRepo.put(savedProject);
  }
}
