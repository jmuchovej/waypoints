import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'project_entity.dart';
import '../task/task_entity.dart';
import '../../objectbox/objectbox.g.dart';
import '../task/task_repository.dart';
import '../thing/interface.dart';

/// Provider for ProjectRepository
final projectRepositoryProvider = Provider<ProjectRepository>((ref) {
  return ProjectRepository(ref);
});

/// Repository for ProjectEntity with specific project queries
class ProjectRepository extends ThingRepository<ProjectEntity> {
  ProjectRepository(super._ref);

  @override
  Box<ProjectEntity> get box => Box<ProjectEntity>(store);

  @override
  String get type => ProjectEntity.key;

  /// Get all active (non-archived) projects
  List<ProjectEntity> active() {
    final query = box.query(ProjectEntity_.archived.equals(false));
    return query.build().find();
  }

  /// Archive a project and all its contents
  void archive(String uuid) {
    final project = getByUUID(uuid);
    if (project == null) return;

    final updatedProject = project.copyWith(archived: true);
    put(updatedProject);

    // Archive all tasks in the project
    final taskRepo = ref.read(taskRepositoryProvider);
    final tasks = taskRepo.inProjectDeep(uuid);
    for (final task in tasks) {
      taskRepo.archive(task.uuid);
    }

    // Archive subprojects
    final childProjects = sameTypedChildren(uuid);
    for (final child in childProjects) {
      archive(child.uuid);
    }
  }

  /// Get all archived projects
  List<ProjectEntity> archived() {
    final query = box.query(ProjectEntity_.archived.equals(true));
    return query.build().find();
  }

  /// Complete a project (marks all tasks as done)
  void complete(String uuid) {
    updateStatus(uuid, ProjectStatus.complete);

    // Complete all tasks
    final taskRepo = ref.read(taskRepositoryProvider);
    final tasks = taskRepo.inProjectDeep(uuid);
    for (final task in tasks) {
      taskRepo.updateStatus(task.uuid, TaskStatus.complete);
    }
  }

  /// Get projects created within a date range
  List<ProjectEntity> createdBetween(DateTime start, DateTime end) {
    final query = box.query(
      ProjectEntity_.createDate.between(
        start.millisecondsSinceEpoch,
        end.millisecondsSinceEpoch,
      ),
    );
    return query.build().find();
  }

  /// Get project statistics
  ProjectStats getStats(String projectUuid) {
    final taskRepo = ref.read(taskRepositoryProvider);
    final tasks = taskRepo.inProjectDeep(projectUuid);

    return ProjectStats(
      totalTasks: tasks.length,
      completedTasks: tasks.where((t) => t.status.isCompleted).length,
      inProgressTasks: tasks.where((t) => t.status.isPending).length,
      blockedTasks: tasks.where((t) => t.status.isAwaiting).length,
      todoTasks: tasks.where((t) => t.status.isPending).length,
      overdueTasks:
          tasks
              .where(
                (t) =>
                    t.dueDate.isBefore(DateTime.now()) && !t.status.isCompleted,
              )
              .length,
    );
  }

  /// Get all top-level projects (no parent)
  List<ProjectEntity> topLevel() {
    return getAll().where((project) => !project.parent.hasValue).toList();
  }

  /// Unarchive a project
  void unarchive(String uuid) {
    final project = getByUUID(uuid);
    if (project == null) return;

    final updatedProject = project.copyWith(archived: false);
    put(updatedProject);
  }

  /// Get projects updated within a date range
  List<ProjectEntity> updatedBetween(DateTime start, DateTime end) {
    final query = box.query(
      ProjectEntity_.modifyDate.between(
        start.millisecondsSinceEpoch,
        end.millisecondsSinceEpoch,
      ),
    );
    return query.build().find();
  }

  /// Update project status
  void updateStatus(String uuid, ProjectStatus status) {
    final project = getByUUID(uuid);
    if (project == null) return;

    final updatedProject = project.copyWith(status: status);
    put(updatedProject);
  }

  /// Get projects by status
  List<ProjectEntity> withStatus(ProjectStatus status) {
    final query = box.query(ProjectEntity_.obxStatus.equals(status.index));
    return query.build().find();
  }
}

/// Statistics for a project
class ProjectStats {
  const ProjectStats({
    required this.totalTasks,
    required this.completedTasks,
    required this.inProgressTasks,
    required this.blockedTasks,
    required this.todoTasks,
    required this.overdueTasks,
  });

  final int totalTasks;
  final int completedTasks;
  final int inProgressTasks;
  final int blockedTasks;
  final int todoTasks;
  final int overdueTasks;

  int get activeTasks => inProgressTasks + todoTasks;

  double get completionPercentage {
    if (totalTasks == 0) return 0.0;
    return (completedTasks / totalTasks) * 100;
  }
}
