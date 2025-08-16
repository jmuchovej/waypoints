import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/foundation.dart';
import '../../../things/project/project.entity.dart';
import '../../../things/task/task.entity.dart';
import '../seed.dart';
import '_seed.dart';

class HomeOfficeRenovationProject extends DatabaseSeed {
  final seedProject = ProjectEntity(
    uuid: "1deb860f-e324-439c-b478-89e16b140373",
    title: AttributedText("Home Office Renovation"),
  );

  final seedDesignTask = DevtoolsTaskDefaults.withDefaults(
    'cf73752b-3d54-4a9b-b463-79b8127fda5a',
    AttributedText('Design and planning'),
    dateDelta: const Duration(days: 14),
  );

  final seedMeasureTask = DevtoolsTaskDefaults.withDefaults(
    '92fc187b-5600-4e07-9578-e2d17f64e686',
    AttributedText('Measure room dimensions'),
    status: TaskStatus.complete,
  );

  final seedMoodboardTask = DevtoolsTaskDefaults.withDefaults(
    'e44b6c08-0784-4feb-9ed7-71b308cd70db',
    AttributedText('Create mood board on Pinterest'),
    status: TaskStatus.workInProgress,
  );

  final seedLayoutTask = DevtoolsTaskDefaults.withDefaults(
    'a4dc450b-adb7-4cb4-b3ac-7238cf1f948b',
    AttributedText('Sketch layout options'),
    status: TaskStatus.started,
  );

  final seedFurnitureTask = DevtoolsTaskDefaults.withDefaults(
    '4b7690fb-025e-448a-bd7d-28695582b92b',
    AttributedText('Purchase furniture and equipment'),
    dateDelta: const Duration(days: 30),
  );

  final seedStandingDeskTask = DevtoolsTaskDefaults.withDefaults(
    '4b6bc533-741c-4861-9608-9ad636ea0d79',
    AttributedText('Standing desk: Compare Uplift vs Fully vs Flexispot'),
  );

  final seedErgonomicChairTask = DevtoolsTaskDefaults.withDefaults(
    'bb11f065-11fb-4ffc-b25c-1ed2970fbaf2',
    AttributedText(
      'Ergonomic chair: Test Herman Miller Aeron vs Steelcase Leap',
    ),
  );

  final seedMonitorArmTask = DevtoolsTaskDefaults.withDefaults(
    '0b8508c6-222d-47ff-9257-ce22d4105ebe',
    AttributedText('Monitor arm for dual 27" displays'),
  );

  final seedCableManagementTask = DevtoolsTaskDefaults.withDefaults(
    'de6d607a-dbe4-448a-9032-20d5d4791ad8',
    AttributedText('Cable management system'),
  );

  @override
  Future<void> seed(SeedDataService s) async {
    waypointsDBSeedLog.finer(
      '  <seeding project="${seedProject.title.toPlainText()}">',
    );
    final project = await s.projectRepo.putAsync(seedProject);

    // Design task and subtasks
    final designTask = await s.taskRepo.putAsync(seedDesignTask);
    final measureTask = await s.taskRepo.putAsync(seedMeasureTask);
    final moodboardTask = await s.taskRepo.putAsync(seedMoodboardTask);
    final layoutTask = await s.taskRepo.putAsync(seedLayoutTask);

    designTask.thing.childrenRef.addAll(
      [measureTask, moodboardTask, layoutTask].things,
    );
    await s.taskRepo.putAsync(designTask);

    // Furniture task and subtasks
    final furnitureTask = await s.taskRepo.putAsync(seedFurnitureTask);
    final standingDeskTask = await s.taskRepo.putAsync(seedStandingDeskTask);
    final ergonomicChairTask = await s.taskRepo.putAsync(
      seedErgonomicChairTask,
    );
    final monitorArmTask = await s.taskRepo.putAsync(seedMonitorArmTask);
    final cableManagementTask = await s.taskRepo.putAsync(
      seedCableManagementTask,
    );

    furnitureTask.thing.childrenRef.addAll(
      [
        standingDeskTask,
        ergonomicChairTask,
        monitorArmTask,
        cableManagementTask,
      ].things,
    );
    await s.taskRepo.putAsync(furnitureTask);

    project.thing.childrenRef.addAll([designTask, furnitureTask].things);
    await s.projectRepo.putAsync(project);
  }
}
