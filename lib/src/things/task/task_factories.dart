import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../factory.docheaders.dart';
import '../factory.node.dart';
import '../../editor/nodes/task_node.dart';
import '../../editor/headers/task_editor_header.dart';
import 'task.entity.dart';
import '../../objectbox/repositories/task_repository.dart';

/// Node factory for converting Task entities to TaskNodes
class TaskHeaderFactory
    extends ThingPanelHeaderFactory<TaskEntity, TaskRepository> {
  TaskHeaderFactory(Ref ref)
    : super(ref, taskRepositoryProvider, TaskEditorHeader.fromTask);

  @override
  String get type => TaskEntity.key;
}

/// Node factory for converting Task entities to TaskNodes
class TaskNodeFactory
    extends ThingNodeFactory<TaskEntity, TaskRepository, TaskNode> {
  TaskNodeFactory(Ref ref)
    : super(ref, taskRepositoryProvider, TaskNode.fromTask);

  @override
  String get type => TaskEntity.key;
}
