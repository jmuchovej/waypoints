import 'package:flutter/foundation.dart';
import 'package:super_editor/super_editor.dart';
import '../../things/foundation.dart';
import '../../things/thing.entity.dart';
import '../../things/project/project.entity.dart';
import '../../objectbox/repositories/project_repository.dart';

const NamedAttribution projectAttribution = NamedAttribution("project");
const BlockSelector projectSelector = BlockSelector("project");
const NamedAttribution projectTitleAttribution = NamedAttribution(
  "projectTitle",
);

/// Styles all task components to apply top padding
final projectStyles = StyleRule(projectSelector, (document, node) {
  if (node is! ProjectNode) {
    return {};
  }

  // return {Styles.padding: const CascadingPadding.only(top: 6)};
  return {};
});

int _computeIndentation(ProjectEntity? project, int indent) {
  if (indent != -1) return indent;

  while (project != null) {
    indent += 1;
  }

  return indent;
}

/// A document node that represents a project in the editor
@immutable
class ProjectNode extends TextNode implements ThingNode {
  factory ProjectNode.fromUUID(
    String uuid,
    ProjectRepository repo, {
    Map<String, dynamic>? metadata,
    int indent = -1,
  }) {
    final project = repo.byUUID(uuid);
    if (project == null) {
      throw StateError('Project with UUID (uuid=$uuid) not found.');
    }

    return ProjectNode.fromProject(project, metadata: metadata, indent: indent);
  }

  /// Factory method to create a ProjectNode from a project entity
  ProjectNode.fromProject(
    this.project, {
    super.metadata,
    int indent = -1,
    this.inEditMode = false,
  }) : indent = _computeIndentation(project, indent),
       super(id: project.uuid, text: project.title) {
    initAddToMetadata({
      'blockType': projectAttribution,
      'inEditMode': inEditMode,
    });
  }

  @override
  String get id => project.uuid;

  /// The project entity this node represents
  late final ProjectEntity project;
  final int indent;
  final bool inEditMode;

  String get type => ProjectEntity.key;
  AttributedText get title => project.title;

  ProjectNode copyProjectWith({
    ProjectEntity? project,
    int? indent,
    bool? inEditMode,
    Map<String, dynamic>? metadata,
  }) {
    return ProjectNode.fromProject(
      (project ?? this.project).copyWith(),
      indent: indent ?? this.indent,
      inEditMode: inEditMode ?? this.inEditMode,
      metadata: metadata ?? this.metadata,
    );
  }

  @override
  ProjectNode copyTextNodeWith({
    String? id,
    AttributedText? text,
    Map<String, dynamic>? metadata,
  }) {
    if (id == null || id == this.id) {
      final updatedProject = project.copyWith(title: text ?? title);
      return ProjectNode.fromProject(
        updatedProject,
        metadata: metadata ?? this.metadata,
      );
    }

    throw UnsupportedError(
      'Cannot change task ID without access to project repository',
    );
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
  int get hashCode => super.hashCode ^ id.hashCode ^ project.uuid.hashCode;

  @override
  ProjectNode copyAndReplaceMetadata(Map<String, dynamic> newMetadata) {
    return copyProjectWith(metadata: newMetadata);
  }

  @override
  ProjectNode copyWithAddedMetadata(Map<String, dynamic> newProperties) {
    return copyProjectWith(metadata: {...metadata, ...newProperties});
  }

  @override
  ThingEntity get thing =>
      ThingEntity(uuid: project.uuid, type: project.type, id: project.id);
}

extension ProjectNodeType on DocumentNode {
  ProjectNode get asProject => this as ProjectNode;
}
