import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../things/task/task_editor.dart';
import '../../things/task/task_repository.dart';

@RoutePage(name: 'TaskEditorRoute')
class TaskEditorScreen extends ConsumerWidget {
  const TaskEditorScreen({super.key, this.uuid});

  final String? uuid;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // If uuid provided, load the task
    if (uuid != null) {
      try {
        final task = ref.read(taskRepositoryProvider).byUUID(uuid!);
        if (task != null) {
          return TaskEditor(task: task, projectID: task.project.targetId);
        } else {
          // Task not found
          return Scaffold(
            appBar: AppBar(title: const Text('Task Not Found')),
            body: const Center(child: Text('Task not found')),
          );
        }
      } catch (e) {
        // Error loading task
        return Scaffold(
          appBar: AppBar(title: const Text('Error')),
          body: Center(child: Text('Error loading task: $e')),
        );
      }
    }

    // Otherwise create new task
    return const TaskEditor();
  }
}
