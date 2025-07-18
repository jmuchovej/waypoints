import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'devtools/screens/devtools.dart';
import 'infrastructure/logging.dart';
import 'infrastructure/route_visibility.dart';
import 'routes/index.dart';
import 'routes/mobile_home.dart';
import 'routes/project/index.dart';
import 'routes/task/index.dart';
import 'routes/thing/uuid.dart';

// go_router configuration
final routerProvider = Provider<GoRouter>((ref) {
  // Determine initial location based on platform
  final screenWidth =
      WidgetsBinding.instance.platformDispatcher.views.first.physicalSize.width;
  final pixelRatio =
      WidgetsBinding.instance.platformDispatcher.views.first.devicePixelRatio;
  final logicalWidth = screenWidth / pixelRatio;
  final isMobile = logicalWidth <= 768;

  return GoRouter(
    initialLocation: isMobile ? '/home' : '/tasks',
    routes: [
      // Main shell route with navigation
      StatefulShellRoute.indexedStack(
        builder: (context, state, navigationShell) {
          return RouteTracker(
            child: IndexPage(navigationShell: navigationShell),
          );
        },
        branches: [
          // Mobile Home branch - main navigation hub
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/home',
                pageBuilder:
                    (context, state) => NoTransitionPage(
                      key: state.pageKey,
                      child: const MobileHomeScreen(),
                    ),
              ),
            ],
          ),
          // Tasks branch - simple navigation
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/tasks',
                pageBuilder:
                    (context, state) => NoTransitionPage(
                      key: state.pageKey,
                      child: const TasksScreen(),
                    ),
              ),
            ],
          ),
          // Projects branch - simple navigation
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/projects',
                pageBuilder:
                    (context, state) => NoTransitionPage(
                      key: state.pageKey,
                      child: const ProjectsScreen(),
                    ),
              ),
            ],
          ),
          // DevTools branch - development tools
          StatefulShellBranch(
            routes: [
              GoRoute(
                path: '/devtools',
                pageBuilder:
                    (context, state) => NoTransitionPage(
                      key: state.pageKey,
                      child: const DevToolsScreen(),
                    ),
              ),
            ],
          ),
        ],
      ),

      // Additional top-level routes
      GoRoute(
        path: '/thing/:uuid',
        builder: (context, state) {
          final uuid = state.pathParameters['uuid']!;
          return ThingPage(uuid: uuid);
        },
      ),
    ],
  );
});

// Custom page transition with no animation
class NoTransitionPage<T> extends Page<T> {
  const NoTransitionPage({
    required this.child,
    super.key,
    super.name,
    super.arguments,
    super.restorationId,
  });

  final Widget child;

  @override
  Route<T> createRoute(BuildContext context) {
    return PageRouteBuilder<T>(
      settings: this,
      pageBuilder: (context, animation, _) => child,
      transitionDuration: Duration.zero,
      reverseTransitionDuration: Duration.zero,
    );
  }
}

// Extension methods for easier navigation with go_router
extension GoRouterNavigation on BuildContext {
  void gotoHome() {
    go('/tasks');
  }

  void gotoTasks() {
    go('/tasks');
  }

  void gotoProjects() {
    go('/projects');
  }

  void gotoDemo() {
    go('/demo');
  }

  void gotoThing(String uuid) {
    go('/thing/$uuid');
  }

  void gotoCreateTask() {
    go('/create/task');
  }

  void gotoCreateProject() {
    go('/create/project');
  }

  // Traditional push navigation - adds to navigation stack
  void pushThing(String uuid) {
    push('/thing/$uuid');
  }

  void pushThingFromEntity(dynamic thing, dynamic ref) {
    if (thing != null && thing.uuid != null) {
      pushThing(thing.uuid);
    }
  }

  // Navigation methods that maintain standalone navigation behavior
  void goThing(dynamic thing, dynamic ref) {
    if (thing != null && thing.uuid != null) {
      go('/thing/${thing.uuid}');
    }
  }

  void goProject(dynamic project, dynamic ref) {
    if (project != null && project.uuid != null) {
      go('/thing/${project.uuid}');
    }
  }
}
