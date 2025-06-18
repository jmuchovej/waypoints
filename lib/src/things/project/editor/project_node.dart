import 'package:super_editor/super_editor.dart';
import '../project_entity.dart';

/// A document node that represents a project in the editor
class ProjectNode extends BlockNode {
  ProjectNode({required this.project});

  /// The project entity this node represents
  final ProjectEntity project;

  String get type => 'project';

  ProjectNode copyWith({ProjectEntity? project}) {
    return ProjectNode(project: project ?? this.project);
  }

  @override
  bool hasEquivalentContent(DocumentNode other) {
    return other is ProjectNode && project.uuid == other.project.uuid;
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ProjectNode &&
          runtimeType == other.runtimeType &&
          id == other.id &&
          project.uuid == other.project.uuid;

  @override
  int get hashCode => id.hashCode ^ project.uuid.hashCode;

  /// Factory method to create a ProjectNode from a project entity
  static ProjectNode fromProject(ProjectEntity project) {
    return ProjectNode(project: project);
  }

  @override
  DocumentNode copyAndReplaceMetadata(Map<String, dynamic> newMetadata) {
    // TODO: implement copyAndReplaceMetadata
    throw UnimplementedError();
  }

  @override
  String? copyContent(NodeSelection selection) {
    // TODO: implement copyContent
    throw UnimplementedError();
  }

  @override
  DocumentNode copyWithAddedMetadata(Map<String, dynamic> newProperties) {
    // TODO: implement copyWithAddedMetadata
    throw UnimplementedError();
  }

  @override
  String get id => project.uuid;
}
