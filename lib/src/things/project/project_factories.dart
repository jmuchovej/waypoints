import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../thing_factories.dart';
import 'project_entity.dart';
import 'project_repository.dart';
import 'editor/project_node.dart';
import 'project_editor_header.dart';

/// Node factory for converting Project entities to ProjectNodes
class ProjectNodeFactory
    extends ThingNodeFactory<ProjectEntity, ProjectRepository, ProjectNode> {
  ProjectNodeFactory(Ref ref)
    : super(ref, projectRepositoryProvider, ProjectNode.fromProject);

  @override
  String get type => ProjectEntity.key;
}

/// Node factory for converting Project entities to ProjectNodes
class ProjectHeaderFactory extends ThingPanelHeaderFactory<ProjectEntity, ProjectRepository> {
  ProjectHeaderFactory(Ref ref) : super(ref, projectRepositoryProvider, ProjectEditorHeader.fromProject);

  @override
  String get type => ProjectEntity.key;
}
