import 'package:flutter/material.dart';
import 'package:tempo/main.dart' show obx;
import 'package:tempo/src/editor/super_editor.dart';

import 'package:tempo/src/things/task.dart';
import 'package:tempo/src/things/_thing/node.dart';

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
    String uuid, {
    Map<String, dynamic>? metadata,
    int indent = -1,
  }) {
    final task = obx.getTaskByUUID(uuid);

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
  String get thingUuid => task.uuid;

  @override
  String get thingType => 'task';

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
    final task = TaskNode.fromUUID(
      id ?? this.id,
    ).task.copyWith(title: text ?? title);
    return TaskNode.fromTask(task, metadata: metadata ?? this.metadata);
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

  @override
  TextNodeSelection computeSelection({
    required NodePosition base,
    required NodePosition extent,
  }) {
    return super.computeSelection(base: base, extent: extent);
  }
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
