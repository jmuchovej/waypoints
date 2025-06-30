import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../node_factory.dart';
import 'project_entity.dart';
import 'project_repository.dart';
import 'editor/project_node.dart';

/// Node factory for converting Project entities to ProjectNodes
class ProjectNodeFactory
    extends ThingNodeFactory<ProjectEntity, ProjectRepository, ProjectNode> {
  ProjectNodeFactory(Ref ref)
    : super(ref, projectRepositoryProvider, ProjectNode.fromProject);

  @override
  String get type => ProjectEntity.key;
}
