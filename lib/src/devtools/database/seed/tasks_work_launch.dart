import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../things/project.dart';
import '../../../things/task/task_entity.dart';
import '../seed.dart';
import '_seed.dart';

class Q1LaunchProject implements DatabaseSeed {
  final project = ProjectEntity(
    title: AttributedText("Q1 Product Launch"),
    uuid: "ad8fd8d8-974c-49ff-bbfc-b474ccca3467",
  );

  final featureDevTask = DevtoolsTaskDefaults.withDefaults(
    "906ed45c-c56d-45a6-b24c-f109f90155d9",
    AttributedText("Complete feature development"),
    status: TaskStatus.workInProgress,
    dateDelta: const Duration(days: -2),
    priority: TaskPriority.H,
  );

  final userAuthTask = DevtoolsTaskDefaults.withDefaults(
    "265a8582-aba3-48a5-97c1-964a8d749db7",
    AttributedText("Implement user authentication flow"),
    status: TaskStatus.complete,
  );

  final analyticsDashboardTask = DevtoolsTaskDefaults.withDefaults(
    "4110eaee-405d-452f-b5bd-9ceac2af094f",
    AttributedText("Build dashboard analytics view"),
  );

  final exportTask = DevtoolsTaskDefaults.withDefaults(
    "01e75f3b-c679-4718-974b-46d3a561a63f",
    AttributedText("Add export functionality"),
    status: TaskStatus.started,
  );

  final paymentTask = DevtoolsTaskDefaults.withDefaults(
    "10189f69-cc34-4ca1-a516-0a3684985ad7",
    AttributedText("Integrate payment processing"),
  );

  final marketingTask = DevtoolsTaskDefaults.withDefaults(
    "1cfa4479-77e1-485e-8a2c-1e358868df8a",
    AttributedText("Prepare marketing materials"),
    status: TaskStatus.started,
    dateDelta: const Duration(days: 7),
    priority: TaskPriority.H,
  );

  final productTask = DevtoolsTaskDefaults.withDefaults(
    "e2f6b210-00f3-4fee-8a01-5aaf553bb633",
    AttributedText("Write product announcement blog post"),
    status: TaskStatus.started,
  );

  final demoTask = DevtoolsTaskDefaults.withDefaults(
    "e8e62978-a036-4901-ad40-b305a60e1110",
    AttributedText("Create demo video script"),
  );

  final emailTask = DevtoolsTaskDefaults.withDefaults(
    "ff4d1126-7ba5-4cb3-8f4c-9b718335ada1",
    AttributedText("Design email campaign templates"),
  );

  @override
  void seed(SeedDataService s) {
    dbLog.finer('  <seeding project="${project.titlePlainText}">');
    final projectID = s.projectRepo.put(project);

    // Feature dev task and subtasks
    final [
      featureDevID,
      userAuthID,
      analyticsDashboardID,
      exportID,
      paymentID,
    ] = s.taskRepo.putMany([
      featureDevTask,
      userAuthTask,
      analyticsDashboardTask,
      exportTask,
      paymentTask,
    ]);

    final savedFeatureDevTask = s.taskRepo.get(featureDevID)!;
    savedFeatureDevTask.children.addAll(
      s.taskRepo
          .getMany([userAuthID, analyticsDashboardID, exportID, paymentID])
          .map((t) => t!.asThingEntity()),
    );
    s.taskRepo.put(savedFeatureDevTask);

    // Marketing task and subtasks
    final [marketingID, productID, demoID, emailID] = s.taskRepo.putMany([
      marketingTask,
      productTask,
      demoTask,
      emailTask,
    ]);

    final savedMarketingTask = s.taskRepo.get(marketingID)!;
    savedMarketingTask.children.addAll(
      s.taskRepo
          .getMany([productID, demoID, emailID])
          .map((t) => t!.asThingEntity()),
    );
    s.taskRepo.put(savedMarketingTask);

    final savedProject = s.projectRepo.get(projectID)!;
    savedProject.children.addAll(
      s.taskRepo
          .getMany([featureDevID, marketingID])
          .map((t) => t!.asThingEntity()),
    );
    s.projectRepo.put(savedProject);
  }
}
