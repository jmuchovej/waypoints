import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:tempo/pages/projects.dart';
import 'package:tempo/pages/task_editor.dart';
import 'package:tempo/modules/flux/pages/tasks.dart';
import 'package:tempo/dev/screens/devtools.dart';
import 'package:tempo/flavors.dart';
import 'package:tempo/main.dart' show obx;
import 'package:tempo/src/things/entities.dart';
import 'package:tempo/src/database/repositories/uuid_repository.dart';

part 'auto_router.gr.dart';

@AutoRouterConfig(replaceInRouteName: 'Page|Screen,Route')
class AppRouter extends RootStackRouter {
  @override
  List<AutoRoute> get routes => [
        AutoRoute(
          path: '/',
          page: HomeShellRoute.page,
          children: [
            AutoRoute(
              path: 'tasks',
              page: TasksRoute.page,
              initial: true,
            ),
            AutoRoute(
              path: 'projects',
              page: ProjectsRoute.page,
            ),
            if (F.appFlavor == Flavor.development)
              AutoRoute(
                path: 'devtools',
                page: DevToolsRoute.page,
              ),
          ],
        ),
        // Entity route with parameter
        AutoRoute(
          path: '/thing/:uuid',
          page: EntityRoute.page,
        ),
      ];

  @override
  RouteType get defaultRouteType => const RouteType.material();
}

// Create router pages with @RoutePage annotation
@RoutePage()
class HomeShellPage extends StatelessWidget {
  const HomeShellPage({super.key});

  @override
  Widget build(BuildContext context) {
    return AutoTabsScaffold(
      routes: [
        const TasksRoute(),
        ProjectsRoute(),
        if (F.appFlavor == Flavor.development) const DevToolsRoute(),
      ],
      bottomNavigationBuilder: (_, tabsRouter) {
        final colorScheme = Theme.of(context).colorScheme;

        return NavigationBar(
          selectedIndex: tabsRouter.activeIndex,
          onDestinationSelected: tabsRouter.setActiveIndex,
          destinations: [
            NavigationDestination(
              icon: Icon(
                Icons.format_list_bulleted_rounded,
                color: colorScheme.primaryContainer,
              ),
              selectedIcon: Icon(
                Icons.format_list_bulleted_rounded,
                color: colorScheme.primary,
              ),
              label: 'Tasks',
            ),
            NavigationDestination(
              icon: Icon(
                Icons.folder_outlined,
                color: colorScheme.primaryContainer,
              ),
              selectedIcon: Icon(
                Icons.folder_rounded,
                color: colorScheme.primary,
              ),
              label: 'Projects',
            ),
            if (F.appFlavor == Flavor.development)
              NavigationDestination(
                icon: Icon(
                  Icons.developer_mode_outlined,
                  color: colorScheme.primaryContainer,
                ),
                selectedIcon: Icon(
                  Icons.developer_mode,
                  color: colorScheme.primary,
                ),
                label: 'Dev Tools',
              ),
          ],
        );
      },
    );
  }
}

@RoutePage()
class EntityPage extends StatelessWidget {
  const EntityPage({
    super.key,
    @PathParam('uuid') required this.uuid,
  });

  final String uuid;

  @override
  Widget build(BuildContext context) {
    // Look up the entity type from the UUID registry
    final registryEntry = UuidService.lookupUuid(uuid);

    if (registryEntry == null) {
      // UUID not found in registry
      return _buildEntityNotFoundScreen(context, uuid);
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

      default:
        // Unknown entity type
        break;
    }

    // If we get here, the entity was found in registry but not in the actual database
    return _buildEntityNotFoundScreen(context, uuid, registryEntry.entityType);
  }

  Widget _buildEntityNotFoundScreen(BuildContext context, String uuid, [String? entityType]) {
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
}

// Extension methods for easier navigation with auto_route
extension AutoRouterNavigation on BuildContext {
  void gotoEntity(String uuid) {
    router.push(EntityRoute(uuid: uuid));
  }

  void gotoTask(TaskEntity task) {
    router.push(EntityRoute(uuid: task.uuid));
  }

  void gotoProject(ProjectEntity project) {
    router.push(EntityRoute(uuid: project.uuid));
  }

  void gotoTasks() {
    router.navigate(const TasksRoute());
  }

  void gotoProjects() {
    router.navigate(ProjectsRoute());
  }

  void gotoHome() {
    router.navigate(const TasksRoute());
  }

  void gotoDevTools() {
    if (F.appFlavor == Flavor.development) {
      router.navigate(const DevToolsRoute());
    }
  }
}
