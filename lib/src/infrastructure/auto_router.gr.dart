// dart format width=80
// GENERATED CODE - DO NOT MODIFY BY HAND

// **************************************************************************
// AutoRouterGenerator
// **************************************************************************

// ignore_for_file: type=lint
// coverage:ignore-file

part of 'auto_router.dart';

/// generated route for
/// [DevToolsScreen]
class DevToolsRoute extends PageRouteInfo<void> {
  const DevToolsRoute({List<PageRouteInfo>? children})
    : super(DevToolsRoute.name, initialChildren: children);

  static const String name = 'DevToolsRoute';

  static PageInfo page = PageInfo(
    name,
    builder: (data) {
      return const DevToolsScreen();
    },
  );
}

/// generated route for
/// [EntityPage]
class EntityRoute extends PageRouteInfo<EntityRouteArgs> {
  EntityRoute({Key? key, required String uuid, List<PageRouteInfo>? children})
    : super(
        EntityRoute.name,
        args: EntityRouteArgs(key: key, uuid: uuid),
        rawPathParams: {'uuid': uuid},
        initialChildren: children,
      );

  static const String name = 'EntityRoute';

  static PageInfo page = PageInfo(
    name,
    builder: (data) {
      final pathParams = data.inheritedPathParams;
      final args = data.argsAs<EntityRouteArgs>(
        orElse: () => EntityRouteArgs(uuid: pathParams.getString('uuid')),
      );
      return EntityPage(key: args.key, uuid: args.uuid);
    },
  );
}

class EntityRouteArgs {
  const EntityRouteArgs({this.key, required this.uuid});

  final Key? key;

  final String uuid;

  @override
  String toString() {
    return 'EntityRouteArgs{key: $key, uuid: $uuid}';
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! EntityRouteArgs) return false;
    return key == other.key && uuid == other.uuid;
  }

  @override
  int get hashCode => key.hashCode ^ uuid.hashCode;
}

/// generated route for
/// [HomeShellPage]
class HomeShellRoute extends PageRouteInfo<void> {
  const HomeShellRoute({List<PageRouteInfo>? children})
    : super(HomeShellRoute.name, initialChildren: children);

  static const String name = 'HomeShellRoute';

  static PageInfo page = PageInfo(
    name,
    builder: (data) {
      return const HomeShellPage();
    },
  );
}

/// generated route for
/// [ProjectsScreen]
class ProjectsRoute extends PageRouteInfo<ProjectsRouteArgs> {
  ProjectsRoute({
    Key? key,
    ProjectEntity? initialProject,
    List<PageRouteInfo>? children,
  }) : super(
         ProjectsRoute.name,
         args: ProjectsRouteArgs(key: key, initialProject: initialProject),
         initialChildren: children,
       );

  static const String name = 'ProjectsRoute';

  static PageInfo page = PageInfo(
    name,
    builder: (data) {
      final args = data.argsAs<ProjectsRouteArgs>(
        orElse: () => const ProjectsRouteArgs(),
      );
      return ProjectsScreen(key: args.key, initialProject: args.initialProject);
    },
  );
}

class ProjectsRouteArgs {
  const ProjectsRouteArgs({this.key, this.initialProject});

  final Key? key;

  final ProjectEntity? initialProject;

  @override
  String toString() {
    return 'ProjectsRouteArgs{key: $key, initialProject: $initialProject}';
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! ProjectsRouteArgs) return false;
    return key == other.key && initialProject == other.initialProject;
  }

  @override
  int get hashCode => key.hashCode ^ initialProject.hashCode;
}

/// generated route for
/// [TasksScreen]
class TasksRoute extends PageRouteInfo<void> {
  const TasksRoute({List<PageRouteInfo>? children})
    : super(TasksRoute.name, initialChildren: children);

  static const String name = 'TasksRoute';

  static PageInfo page = PageInfo(
    name,
    builder: (data) {
      return const TasksScreen();
    },
  );
}
