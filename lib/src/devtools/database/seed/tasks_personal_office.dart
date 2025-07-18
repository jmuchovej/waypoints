import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project/project.entity.dart';
import '../../../things/task/task.entity.dart';
import '../../../things/thing/thing.entity.dart';
import '../seed.dart';
import '_seed.dart';

class HomeOfficeRenovationProject extends DatabaseSeed {
  final project = ProjectEntity(
    uuid: "1deb860f-e324-439c-b478-89e16b140373",
    title: AttributedText("Home Office Renovation"),
  );

  final designTask = DevtoolsTaskDefaults.withDefaults(
    'cf73752b-3d54-4a9b-b463-79b8127fda5a',
    AttributedText('Design and planning'),
    dateDelta: const Duration(days: 14),
  );

  final measureTask = DevtoolsTaskDefaults.withDefaults(
    '92fc187b-5600-4e07-9578-e2d17f64e686',
    AttributedText('Measure room dimensions'),
    status: TaskStatus.complete,
  );

  final moodboardTask = DevtoolsTaskDefaults.withDefaults(
    'e44b6c08-0784-4feb-9ed7-71b308cd70db',
    AttributedText('Create mood board on Pinterest'),
    status: TaskStatus.workInProgress,
  );

  final layoutTask = DevtoolsTaskDefaults.withDefaults(
    'a4dc450b-adb7-4cb4-b3ac-7238cf1f948b',
    AttributedText('Sketch layout options'),
    status: TaskStatus.started,
  );

  final furnitureTask = DevtoolsTaskDefaults.withDefaults(
    '4b7690fb-025e-448a-bd7d-28695582b92b',
    AttributedText('Purchase furniture and equipment'),
    dateDelta: const Duration(days: 30),
  );

  final standingDeskTask = DevtoolsTaskDefaults.withDefaults(
    '4b6bc533-741c-4861-9608-9ad636ea0d79',
    AttributedText('Standing desk: Compare Uplift vs Fully vs Flexispot'),
  );

  final ergonomicChairTask = DevtoolsTaskDefaults.withDefaults(
    'bb11f065-11fb-4ffc-b25c-1ed2970fbaf2',
    AttributedText(
      'Ergonomic chair: Test Herman Miller Aeron vs Steelcase Leap',
    ),
  );

  final monitorArmTask = DevtoolsTaskDefaults.withDefaults(
    '0b8508c6-222d-47ff-9257-ce22d4105ebe',
    AttributedText('Monitor arm for dual 27" displays'),
  );

  final cableManagementTask = DevtoolsTaskDefaults.withDefaults(
    'de6d607a-dbe4-448a-9032-20d5d4791ad8',
    AttributedText('Cable management system'),
  );

  @override
  void seed(SeedDataService s) {
    tempoDBSeedLog.finer(
      '  <seeding project="${project.title.toPlainText()}">',
    );
    final projectID = s.projectRepo.put(project);

    // Design task and subtasks
    final [designID, measureID, moodboardID, layoutID] = s.taskRepo.putMany([
      designTask,
      measureTask,
      moodboardTask,
      layoutTask,
    ]);

    // Set project relationships for design tasks
    final savedProject = s.projectRepo.get(projectID)!;
    tempoDBSeedLog.fine(
      'Setting project relationships for ${savedProject.title.toPlainText()}',
    );
    for (final taskID in [designID, measureID, moodboardID, layoutID]) {
      final task = s.taskRepo.get(taskID)!;
      task.project.target = savedProject;
      s.taskRepo.put(task);
      tempoDBSeedLog.fine(
        'Set project for task ${task.uuid}: ${task.title.toPlainText()} -> ${task.project.target?.title.toPlainText()}',
      );
    }

    final savedDesignTask = s.taskRepo.get(designID)!;
    final savedDesignTaskThingEntity = savedDesignTask.thing.target!;
    savedDesignTaskThingEntity.children.addAll(
      s.taskRepo
          .getMany([measureID, moodboardID, layoutID])
          .map((t) => t!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(savedDesignTaskThingEntity);
    s.taskRepo.put(savedDesignTask);

    // Furniture task and subtasks
    final [
      furnitureID,
      standingDeskID,
      ergonomicChairID,
      monitorArmID,
      cableManagementID,
    ] = s.taskRepo.putMany([
      furnitureTask,
      standingDeskTask,
      ergonomicChairTask,
      monitorArmTask,
      cableManagementTask,
    ]);

    // Set project relationships for furniture tasks
    for (final taskID in [
      furnitureID,
      standingDeskID,
      ergonomicChairID,
      monitorArmID,
      cableManagementID,
    ]) {
      final task = s.taskRepo.get(taskID)!;
      task.project.target = savedProject;
      s.taskRepo.put(task);
      tempoDBSeedLog.fine(
        'Set project for task ${task.uuid}: ${task.title.toPlainText()} -> ${task.project.target?.title.toPlainText()}',
      );
    }

    final savedFurnitureTask = s.taskRepo.get(furnitureID)!;
    final savedFurnitureTaskThingEntity = savedFurnitureTask.thing.target!;
    savedFurnitureTaskThingEntity.children.addAll(
      s.taskRepo
          .getMany([
            standingDeskID,
            ergonomicChairID,
            monitorArmID,
            cableManagementID,
          ])
          .map((t) => t!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.taskRepo.store.box<ThingEntity>().put(savedFurnitureTaskThingEntity);
    s.taskRepo.put(savedFurnitureTask);

    final savedProjectThingEntity = savedProject.thing.target!;
    savedProjectThingEntity.children.addAll(
      s.taskRepo.getMany([designID, furnitureID]).map((t) => t!.thing.target!),
    );
    // Save the ThingEntity to persist the children relationships
    s.projectRepo.store.box<ThingEntity>().put(savedProjectThingEntity);
    s.projectRepo.put(savedProject);
  }
}
