import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:tempo/main.dart' show obx;
import 'package:tempo/pages/home.dart';
import 'package:tempo/pages/projects.dart';
import 'package:tempo/pages/task_editor.dart';
import 'package:tempo/modules/flux/pages/tasks.dart';
import 'package:tempo/src/database/objectbox/models/task.dart';
import 'package:tempo/src/database/objectbox/models/project.dart';
import 'package:tempo/dev/screens/devtools.dart';
import 'package:tempo/flavors.dart';
import 'package:tempo/src/database/repositories/uuid_repository.dart';

final GoRouter router = GoRouter(
  initialLocation: '/scaffolding-demo',
  routes: [
    ShellRoute(
      builder: (context, state, child) => HomeShell(child: child),
      routes: [
        GoRoute(
          path: '/tasks',
          name: 'tasks',
          builder: (context, state) => const TasksScreen(),
        ),
        GoRoute(
          path: '/projects',
          name: 'projects',
          builder: (context, state) => const ProjectsScreen(),
        ),
        if (F.appFlavor == Flavor.development) ...[
          GoRoute(
            path: '/devtools',
            name: 'devtools',
            builder: (context, state) => const DevToolsScreen(),
          ),
        ],
      ],
    ),
    // Entity route with subpath - prevents conflicts with top-level routes
    GoRoute(
      path: '/thing/:uuid',
      name: 'entity',
      builder: (context, state) {
        final uuid = state.pathParameters['uuid']!;
        return _buildEntityScreen(uuid);
      },
    ),
  ],
  errorBuilder:
      (context, state) => Scaffold(
        body: Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Icon(Icons.error_outline, size: 48, color: Colors.red),
              const SizedBox(height: 16),
              Text(
                'Page not found',
                style: Theme.of(context).textTheme.headlineSmall,
              ),
              const SizedBox(height: 8),
              Text(
                state.uri.toString(),
                style: Theme.of(context).textTheme.bodyMedium,
              ),
              const SizedBox(height: 16),
              ElevatedButton(
                onPressed: () => context.go('/'),
                child: const Text('Go Home'),
              ),
            ],
          ),
        ),
      ),
);

// Entity lookup function - determines entity type by UUID and returns appropriate screen
Widget _buildEntityScreen(String uuid) {
  // Look up the entity type from the UUID registry
  final registryEntry = UuidService.lookupUuid(uuid);

  if (registryEntry == null) {
    // UUID not found in registry
    return _buildEntityNotFoundScreen(uuid);
  }

  // Route to appropriate screen based on entity type
  switch (registryEntry.entityType) {
    case 'task':
      try {
        final task = obx.getTaskByUUID(uuid);
        return TaskEditorScreen(task: task);
      } catch (e) {
        // Task not found - possibly deleted but registry not updated
      }
      break;

    case 'project':
      try {
        final project = obx.getProjectByUUID(uuid);
        if (project != null) {
          // For now, redirect to projects page
          // In the future, you could have a ProjectEditorScreen
          return const ProjectsScreen();
        }
      } catch (e) {
        // Project not found - possibly deleted but registry not updated
      }
      break;

    // TODO: Add other entity types here as your system grows
    // case 'note':
    //   try {
    //     final note = obx.getNoteByUUID(uuid);
    //     if (note != null) {
    //       return NoteEditorScreen(note: note);
    //     }
    //   } catch (e) {
    //     // Note not found
    //   }
    //   break;

    default:
      // Unknown entity type
      break;
  }

  // If we get here, the entity was found in registry but not in the actual database
  return _buildEntityNotFoundScreen(uuid, registryEntry.entityType);
}

Widget _buildEntityNotFoundScreen(String uuid, [String? entityType]) {
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
          Builder(
            builder:
                (context) => ElevatedButton(
                  onPressed: () => GoRouter.of(context).go('/tasks'),
                  child: const Text('Go to Tasks'),
                ),
          ),
        ],
      ),
    ),
  );
}

// Extension methods for easier navigation
extension GoRouterNavigation on BuildContext {
  void goToEntity(String uuid) {
    GoRouter.of(this).go('/thing/$uuid');
  }

  void goToTask(TaskEntity task) {
    GoRouter.of(this).go('/thing/${task.uuid}');
  }

  void goToProject(ProjectEntity project) {
    GoRouter.of(this).go('/thing/${project.uuid}');
  }

  void goToTasks() {
    GoRouter.of(this).go('/tasks');
  }

  void goToProjects() {
    GoRouter.of(this).go('/projects');
  }

  void goHome() {
    GoRouter.of(this).go('/tasks');
  }
}
