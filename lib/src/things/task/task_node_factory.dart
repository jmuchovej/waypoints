import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../node_factory.dart';
import 'task_entity.dart';
import 'task_repository.dart';
import 'editor/task_node.dart';

/// Node factory for converting Task entities to TaskNodes
class TaskNodeFactory
    extends ThingNodeFactory<TaskEntity, TaskRepository, TaskNode> {
  TaskNodeFactory(Ref ref)
    : super(ref, taskRepositoryProvider, TaskNode.fromTask);

  @override
  String get type => TaskEntity.key;
}
