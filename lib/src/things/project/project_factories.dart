import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../factory.docheaders.dart';
import '../factory.node.dart';
import 'project.entity.dart';
import 'project_repository.dart';
import '../../editor/nodes/project_node.dart';
import '../../editor/headers/project_editor_header.dart';

/// Node factory for converting Project entities to ProjectNodes
class ProjectNodeFactory
    extends ThingNodeFactory<ProjectEntity, ProjectRepository, ProjectNode> {
  ProjectNodeFactory(Ref ref)
    : super(ref, projectRepositoryProvider, ProjectNode.fromProject);

  @override
  String get type => ProjectEntity.key;
}

/// Node factory for converting Project entities to ProjectNodes
class ProjectHeaderFactory
    extends ThingPanelHeaderFactory<ProjectEntity, ProjectRepository> {
  ProjectHeaderFactory(Ref ref)
    : super(ref, projectRepositoryProvider, ProjectEditorHeader.fromProject);

  @override
  String get type => ProjectEntity.key;
}
