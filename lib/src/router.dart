import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';

import 'devtools/screens/devtools.dart';
import '../flavors.dart';
import 'infrastructure/logging.dart';
import 'ui/shell/pane_manager.dart';
import 'things/project/project_entity.dart';
import 'things/task/task_entity.dart';
import 'things/project.dart';
import 'routes/index.dart';
import 'routes/project/create.dart';
import 'routes/project/detail.dart';
import 'routes/project/index.dart';
import 'routes/task/create.dart';
import 'routes/task/edit.dart';
import 'routes/task/index.dart';
import 'routes/task/today.dart';
import 'routes/thing/[uuid].dart';

part 'router.gr.dart';

// Custom transition builder for no animations
Widget _noTransition(
  BuildContext context,
  Animation<double> animation,
  Animation<double> secondaryAnimation,
  Widget child,
) {
  return child;
}

@AutoRouterConfig(replaceInRouteName: 'Page|Screen,Route')
class AppRouter extends RootStackRouter {
  @override
  RouteType get defaultRouteType => const RouteType.material();

  @override
  List<AutoRoute> get routes => [
    AutoRoute(
      path: '/',
      page: IndexRoute.page,
      children: [
        AutoRoute(
          path: 'tasks',
          page: TasksRoute.page,
          initial: true,
          type: RouteType.custom(
            transitionsBuilder: _noTransition,
            duration: const Duration(milliseconds: 0),
          ),
        ),
        AutoRoute(
          path: 'projects',
          page: ProjectsRoute.page,
          type: RouteType.custom(
            transitionsBuilder: _noTransition,
            duration: const Duration(milliseconds: 0),
          ),
        ),
        if (F.appFlavor == Flavor.development)
          AutoRoute(
            path: 'devtools',
            page: DevToolsRoute.page,
            type: RouteType.custom(
              transitionsBuilder: _noTransition,
              duration: const Duration(milliseconds: 0),
            ),
          ),
      ],
    ),
    // Entity route with parameter (legacy support)
    AutoRoute(path: '/thing/:uuid', page: ThingRoute.page),
    // Project detail route
    AutoRoute(path: '/project/:uuid', page: ProjectDetailRoute.page),
    // Creation routes (full-screen)
    AutoRoute(path: '/create/project', page: CreateProjectRoute.page),
    AutoRoute(path: '/create/task', page: CreateTaskRoute.page),
  ];
}

// Router pages are now defined as separate wrapper classes

// Extension methods for easier navigation with auto_route
extension AutoRouterNavigation on BuildContext {
  void gotoDevTools() {
    if (F.appFlavor == Flavor.development) {
      router.navigate(const DevToolsRoute());
    }
  }

  void gotoEntity(String uuid) {
    router.push(ThingRoute(uuid: uuid));
  }

  void gotoHome() {
    router.navigate(const TasksRoute());
  }

  void gotoProject(ProjectEntity project) {
    tempoLog.fine(
      'gotoProject called with project: ${project.titlePlainText}, UUID: ${project.uuid}',
    );

    // Check if we're on mobile or desktop
    final screenWidth = MediaQuery.of(this).size.width;
    final isMobile = screenWidth <= 768;

    if (isMobile) {
      // On mobile, navigate normally (full screen)
      router.navigate(ProjectDetailRoute(uuid: project.uuid));
      tempoLog.fine(
        'Mobile: Navigation navigated to ProjectDetailContentRoute',
      );
    } else {
      // On desktop, open in detail pane
      paneManager.openDetailPane(
        'project_detail',
        args: {'uuid': project.uuid},
      );
      tempoLog.fine('Desktop: Opened project in detail pane');
    }
  }

  void gotoProjects() {
    router.navigate(const ProjectsRoute());
  }

  void gotoTask(TaskEntity task) {
    router.push(ThingRoute(uuid: task.uuid));
  }

  void gotoTasks() {
    router.navigate(const TasksRoute());
  }
}
