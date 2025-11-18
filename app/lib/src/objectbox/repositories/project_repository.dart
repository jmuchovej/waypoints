import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../objectbox.dart';
import 'thing_repository.dart';
import '../../things/project/project.entity.dart';
import '../../things/task/task.entity.dart';
import '../objectbox.g.dart';
import 'task_repository.dart';

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

  Condition<ProjectEntity> archived(bool archived) {
    return ProjectOBX.archived.equals(archived);
  }

  Condition<ProjectEntity> deleted(bool archived) {
    return ProjectOBX.deleted.equals(archived);
  }

  Condition<ProjectEntity> active() {
    return archived(false).and(deleted(false));
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
      ProjectOBX.createDate.between(
        start.millisecondsSinceEpoch,
        end.millisecondsSinceEpoch,
      ),
    );
    return query.build().find();
  }

  /// Get all top-level projects (no parent)
  List<ProjectEntity> topLevel() {
    return getAll().where((project) => project.parent != null).toList();
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
      ProjectOBX.modifyDate.between(
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

  /// Get all projects with a specific status
  Condition<ProjectEntity> withStatus(ProjectStatus status) {
    return ProjectOBX.statusOBX.equals(status.index);
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
