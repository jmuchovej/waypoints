import 'package:flutter/material.dart';
import '../../../editor/editor.dart';
import '../../../things/task/task_entity.dart';

class ProjectTasksPanel extends StatelessWidget {
  const ProjectTasksPanel({
    super.key,
    required this.tasks,
    required this.tasksDocument,
    required this.tasksComposer,
    required this.hoverManager,
    required this.linkManager,
    required this.onAddTask,
  });

  final List<TaskEntity> tasks;
  final MutableDocument tasksDocument;
  final MutableDocumentComposer tasksComposer;
  final HoverManager hoverManager;
  final LeaderLinkManager linkManager;
  final VoidCallback onAddTask;

  @override
  Widget build(BuildContext context) {
    if (tasks.isEmpty) {
      return _buildEmptyState(context);
    }

    // Use SuperEditor to render tasks
    return createSuperEditor(
      context,
      tasksDocument,
      tasksComposer,
      hoverManager: hoverManager,
      linkManager: linkManager,
    );
  }

  Widget _buildEmptyState(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.task_alt_rounded,
            size: 64,
            color: Theme.of(
              context,
            ).colorScheme.onSurface.withValues(alpha: 0.3),
          ),
          const SizedBox(height: 16),
          Text(
            'No tasks in this project',
            style: TextStyle(
              fontSize: 18,
              color: Theme.of(
                context,
              ).colorScheme.onSurface.withValues(alpha: 0.5),
            ),
          ),
          const SizedBox(height: 24),
          ElevatedButton.icon(
            onPressed: onAddTask,
            icon: const Icon(Icons.add),
            label: const Text('Add First Task'),
          ),
        ],
      ),
    );
  }
}
