// dart format width=80
// GENERATED CODE - DO NOT MODIFY BY HAND

// **************************************************************************
// AutoRouterGenerator
// **************************************************************************

// ignore_for_file: type=lint
// coverage:ignore-file

part of 'router.dart';

/// generated route for
/// [CreateProjectScreen]
class CreateProjectRoute extends PageRouteInfo<void> {
  const CreateProjectRoute({List<PageRouteInfo>? children})
    : super(CreateProjectRoute.name, initialChildren: children);

  static const String name = 'CreateProjectRoute';

  static PageInfo page = PageInfo(
    name,
    builder: (data) {
      return const CreateProjectScreen();
    },
  );
}

/// generated route for
/// [CreateTaskScreen]
class CreateTaskRoute extends PageRouteInfo<CreateTaskRouteArgs> {
  CreateTaskRoute({Key? key, int? projectId, List<PageRouteInfo>? children})
    : super(
        CreateTaskRoute.name,
        args: CreateTaskRouteArgs(key: key, projectId: projectId),
        initialChildren: children,
      );

  static const String name = 'CreateTaskRoute';

  static PageInfo page = PageInfo(
    name,
    builder: (data) {
      final args = data.argsAs<CreateTaskRouteArgs>(
        orElse: () => const CreateTaskRouteArgs(),
      );
      return CreateTaskScreen(key: args.key, projectId: args.projectId);
    },
  );
}

class CreateTaskRouteArgs {
  const CreateTaskRouteArgs({this.key, this.projectId});

  final Key? key;

  final int? projectId;

  @override
  String toString() {
    return 'CreateTaskRouteArgs{key: $key, projectId: $projectId}';
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! CreateTaskRouteArgs) return false;
    return key == other.key && projectId == other.projectId;
  }

  @override
  int get hashCode => key.hashCode ^ projectId.hashCode;
}

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
class ThingRoute extends PageRouteInfo<ThingRouteArgs> {
  ThingRoute({Key? key, required String uuid, List<PageRouteInfo>? children})
    : super(
        ThingRoute.name,
        args: ThingRouteArgs(key: key, uuid: uuid),
        rawPathParams: {'uuid': uuid},
        initialChildren: children,
      );

  static const String name = 'ThingRoute';

  static PageInfo page = PageInfo(
    name,
    builder: (data) {
      final pathParams = data.inheritedPathParams;
      final args = data.argsAs<ThingRouteArgs>(
        orElse: () => ThingRouteArgs(uuid: pathParams.getString('uuid')),
      );
      return EntityPage(key: args.key, uuid: args.uuid);
    },
  );
}

class ThingRouteArgs {
  const ThingRouteArgs({this.key, required this.uuid});

  final Key? key;

  final String uuid;

  @override
  String toString() {
    return 'ThingRouteArgs{key: $key, uuid: $uuid}';
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! ThingRouteArgs) return false;
    return key == other.key && uuid == other.uuid;
  }

  @override
  int get hashCode => key.hashCode ^ uuid.hashCode;
}

/// generated route for
/// [IndexPage]
class IndexRoute extends PageRouteInfo<void> {
  const IndexRoute({List<PageRouteInfo>? children})
    : super(IndexRoute.name, initialChildren: children);

  static const String name = 'IndexRoute';

  static PageInfo page = PageInfo(
    name,
    builder: (data) {
      return const IndexPage();
    },
  );
}

/// generated route for
/// [ProjectDetailPage]
class ProjectDetailRoute extends PageRouteInfo<ProjectDetailRouteArgs> {
  ProjectDetailRoute({
    Key? key,
    required String uuid,
    List<PageRouteInfo>? children,
  }) : super(
         ProjectDetailRoute.name,
         args: ProjectDetailRouteArgs(key: key, uuid: uuid),
         rawPathParams: {'uuid': uuid},
         initialChildren: children,
       );

  static const String name = 'ProjectDetailRoute';

  static PageInfo page = PageInfo(
    name,
    builder: (data) {
      final pathParams = data.inheritedPathParams;
      final args = data.argsAs<ProjectDetailRouteArgs>(
        orElse:
            () => ProjectDetailRouteArgs(uuid: pathParams.getString('uuid')),
      );
      return ProjectDetailPage(key: args.key, uuid: args.uuid);
    },
  );
}

class ProjectDetailRouteArgs {
  const ProjectDetailRouteArgs({this.key, required this.uuid});

  final Key? key;

  final String uuid;

  @override
  String toString() {
    return 'ProjectDetailRouteArgs{key: $key, uuid: $uuid}';
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! ProjectDetailRouteArgs) return false;
    return key == other.key && uuid == other.uuid;
  }

  @override
  int get hashCode => key.hashCode ^ uuid.hashCode;
}

/// generated route for
/// [ProjectsPage]
class ProjectsRoute extends PageRouteInfo<void> {
  const ProjectsRoute({List<PageRouteInfo>? children})
    : super(ProjectsRoute.name, initialChildren: children);

  static const String name = 'ProjectsRoute';

  static PageInfo page = PageInfo(
    name,
    builder: (data) {
      return const ProjectsPage();
    },
  );
}

/// generated route for
/// [TaskEditorScreen]
class TaskEditorRoute extends PageRouteInfo<TaskEditorRouteArgs> {
  TaskEditorRoute({Key? key, String? uuid, List<PageRouteInfo>? children})
    : super(
        TaskEditorRoute.name,
        args: TaskEditorRouteArgs(key: key, uuid: uuid),
        initialChildren: children,
      );

  static const String name = 'TaskEditorRoute';

  static PageInfo page = PageInfo(
    name,
    builder: (data) {
      final args = data.argsAs<TaskEditorRouteArgs>(
        orElse: () => const TaskEditorRouteArgs(),
      );
      return TaskEditorScreen(key: args.key, uuid: args.uuid);
    },
  );
}

class TaskEditorRouteArgs {
  const TaskEditorRouteArgs({this.key, this.uuid});

  final Key? key;

  final String? uuid;

  @override
  String toString() {
    return 'TaskEditorRouteArgs{key: $key, uuid: $uuid}';
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! TaskEditorRouteArgs) return false;
    return key == other.key && uuid == other.uuid;
  }

  @override
  int get hashCode => key.hashCode ^ uuid.hashCode;
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

/// generated route for
/// [TasksTodayPage]
class TasksTodayRoute extends PageRouteInfo<void> {
  const TasksTodayRoute({List<PageRouteInfo>? children})
    : super(TasksTodayRoute.name, initialChildren: children);

  static const String name = 'TasksTodayRoute';

  static PageInfo page = PageInfo(
    name,
    builder: (data) {
      return const TasksTodayPage();
    },
  );
}
