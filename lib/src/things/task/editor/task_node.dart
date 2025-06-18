import 'package:flutter/material.dart';
import '../../../editor/editor.dart';
import '../../thing/interface.dart';
import '../../thing/thing_entity.dart';
import '../task_entity.dart';
import '../task_repository.dart';

const NamedAttribution taskAttribution = NamedAttribution("task");
const BlockSelector taskSelector = BlockSelector("task");
const NamedAttribution taskTitleAttribution = NamedAttribution("taskTitle");
const NamedAttribution taskDescriptionAttribution = NamedAttribution(
  "taskDescription",
);

/// Styles all task components to apply top padding
final taskStyles = StyleRule(taskSelector, (document, node) {
  if (node is! TaskNode) {
    return {};
  }

  // return {Styles.padding: const CascadingPadding.only(top: 6)};
  return {};
});

int _computeIndentation(TaskEntity? task, int indent) {
  if (indent != -1) return indent;

  indent = 0;
  while (task != null && task.parent.hasValue) {
    indent += 1;
    task = task.parent.target;
  }

  return indent;
}

@immutable
class TaskNode extends TextNode implements ThingNode {
  factory TaskNode.fromUUID(
    String uuid,
    TaskRepository taskRepository, {
    Map<String, dynamic>? metadata,
    int indent = -1,
  }) {
    final task = taskRepository.byUUID(uuid);
    if (task == null) {
      throw StateError('Task with UUID $uuid not found');
    }

    return TaskNode.fromTask(task, metadata: metadata, indent: indent);
  }

  TaskNode.fromTask(
    this.task, {
    super.metadata,
    int indent = -1,
    this.inEditMode = false,
  }) : indent = _computeIndentation(task, indent),
       super(id: task.uuid, text: task.title) {
    // TODO determine how to impute the indentation on-creation
    initAddToMetadata({"blockType": taskAttribution, "inEditMode": inEditMode});
  }

  late final TaskEntity task;
  final int indent;
  final bool inEditMode;

  @override
  String get id => task.uuid;

  AttributedText get title => task.title;
  TaskStatus get status => task.status;
  DateTime get dueDate => task.dueDate;
  TaskPriority get priority => task.priority;
  TaskTimeEstimate get timeEstimate => task.timeEstimate;
  TaskEnergyEstimate get energyEstimate => task.energyEstimate;

  @override
  ThingEntity get thing =>
      ThingEntity(uuid: task.uuid, type: task.type, id: task.id);

  TaskNode copyTaskWith({
    TaskEntity? task,
    int? indent,
    bool? inEditMode,
    Map<String, dynamic>? metadata,
  }) {
    return TaskNode.fromTask(
      (task ?? this.task).copyWith(),
      indent: indent ?? this.indent,
      inEditMode: inEditMode ?? this.inEditMode,
      metadata: metadata ?? this.metadata,
    );
  }

  @override
  TaskNode copyTextNodeWith({
    String? id,
    AttributedText? text,
    Map<String, dynamic>? metadata,
  }) {
    // If only text is changing, update the current task
    if (id == null || id == this.id) {
      final updatedTask = task.copyWith(title: text ?? title);
      return TaskNode.fromTask(
        updatedTask,
        metadata: metadata ?? this.metadata,
      );
    }

    // If ID is changing, we can't lookup a different task without a repository
    // This should generally not happen in normal usage
    throw UnsupportedError(
      'Cannot change task ID without access to task repository',
    );
  }

  @override
  bool hasEquivalentContent(DocumentNode other) {
    return other is TaskNode && status == other.status && text == other.text;
  }

  @override
  TaskNode copyWithAddedMetadata(Map<String, dynamic> newProperties) {
    return copyTaskWith(metadata: {...metadata, ...newProperties});
  }

  @override
  DocumentNode copyAndReplaceMetadata(Map<String, dynamic> newMetadata) {
    return copyTaskWith(metadata: newMetadata);
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TaskNode && runtimeType == other.runtimeType && id == other.id;

  @override
  int get hashCode => super.hashCode ^ task.hashCode;
}

extension TaskNodeType on DocumentNode {
  TaskNode get asTask => this as TaskNode;
}

// class TaskNodePosition extends TextPosition implements NodePosition {
//   TaskNodePosition.fromTaskPosition(TextPosition position)
//     : super(offset: position.offset, affinity: position.affinity);
//
//   TaskNodePosition({
//     required int offset,
//     TextAffinity affinity = TextAffinity.downstream,
//   }) : super(offset: offset, affinity: affinity);
//
//   @override
//   bool isEquivalentTo(NodePosition other) {
//     if (other is! TaskNodePosition) {
//       return false;
//     }
//
//     return offset == other.offset;
//   }
//
//   @override
//   int get hashCode => super.hashCode ^ super.offset.hashCode;
//
//   @override
//   bool operator ==(Object other) {
//     return identical(this, other);
//   }
// }
//
// class TaskSelection implements NodeSelection {}
