import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../router.dart';
import '../../things/task/task_repository.dart';
import '../../things/project/project_repository.dart';
import '../../objectbox/services/thing_service.dart';
import '../../things/task.dart';
import '../../things/project.dart';

@RoutePage(name: 'ThingRoute')
class EntityPage extends ConsumerWidget {
  const EntityPage({super.key, @PathParam('uuid') required this.uuid});

  final String uuid;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Look up the entity from the thing service
    final thingService = ref.read(thingServiceProvider);
    final thingEntry = thingService.lookup(uuid);

    if (thingEntry == null) {
      // UUID not found in registry
      return _buildEntityNotFoundScreen(context, uuid);
    }

    // Return appropriate editor based on entity type
    switch (thingEntry.type) {
      case 'task':
        try {
          final task = ref.read(taskRepositoryProvider).getByUUID(uuid);
          if (task != null) {
            return TaskEditor(task: task);
          }
        } catch (e) {
          // Task not found - possibly deleted but registry not updated
        }
        break;

      case 'project':
        try {
          final project = ref.read(projectRepositoryProvider).getByUUID(uuid);
          if (project != null) {
            return ProjectEditor(project: project);
          }
        } catch (e) {
          // Project not found - possibly deleted but registry not updated
        }
        break;

      case 'activity':
        // TODO: Implement ActivityEditor when needed
        return _buildUnsupportedTypeScreen(context, 'activity');

      default:
        // Unknown entity type
        return _buildUnsupportedTypeScreen(context, thingEntry.type);
    }

    // If we get here, the entity was found in registry but not in the actual database
    return _buildEntityNotFoundScreen(context, uuid, thingEntry.type);
  }

  Widget _buildEntityNotFoundScreen(
    BuildContext context,
    String uuid, [
    String? entityType,
  ]) {
    return Scaffold(
      appBar: AppBar(title: const Text('Entity Not Found')),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(Icons.search_off, size: 64, color: Colors.grey),
            const SizedBox(height: 16),
            const Text(
              'Entity not found',
              style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 8),
            Text('UUID: $uuid', style: const TextStyle(color: Colors.grey)),
            if (entityType != null) ...[
              const SizedBox(height: 4),
              Text(
                'Type: $entityType',
                style: const TextStyle(color: Colors.grey),
              ),
            ],
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: () => context.router.replaceAll([const TasksRoute()]),
              child: const Text('Go to Tasks'),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildUnsupportedTypeScreen(BuildContext context, String entityType) {
    return Scaffold(
      appBar: AppBar(title: const Text('Unsupported Entity')),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(Icons.extension_off, size: 64, color: Colors.grey),
            const SizedBox(height: 16),
            const Text(
              'Entity type not supported',
              style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 8),
            Text(
              'Type: $entityType',
              style: const TextStyle(color: Colors.grey),
            ),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: () => context.router.replaceAll([const TasksRoute()]),
              child: const Text('Go to Tasks'),
            ),
          ],
        ),
      ),
    );
  }
}
