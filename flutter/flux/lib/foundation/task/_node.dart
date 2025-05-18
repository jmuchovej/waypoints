import 'package:flutter/material.dart';
import 'package:flux/app.dart';
import 'package:flux/foundation/task.dart';
import 'package:flux/editor.dart';

@immutable
class TaskNode extends TextNode {
  late final int indent;
  late final TaskEntity task;

  factory TaskNode.fromUUID(
    String uuid, {
    Map<String, dynamic>? metadata,
    int indent = -1,
  }) {
    final task = obx.getTaskByUUID(uuid);

    return TaskNode.fromTask(task, metadata: metadata, indent: indent);
  }

  TaskNode.fromTask(this.task, {super.metadata, int indent = -1})
    : super(id: task.uuid, text: AttributedText(task.name)) {
    // TODO determine how to impute the indentation on-creation
    if (indent == -1) {
      indent = 0;
      if (task.parent.hasValue) {
        TaskEntity? task = this.task;
        while (task != null && task.parent.hasValue) {
          indent += 1;
          task = task.parent.target;
        }
      }
    }
    this.indent = indent;
    initAddToMetadata({"blockType": taskAttribution});
  }

  @override
  String get id => task.uuid;

  AttributedText get name => AttributedText(task.name);

  AttributedText get description => AttributedText(task.description);
  TaskStatus get status => task.status;
  DateTime get dueDate => task.dueDate;
  TaskPriority get priority => task.priority;
  TaskTimeEstimate get timeEstimate => task.timeEstimate;
  TaskEnergyEstimate get energyEstimate => task.energyEstimate;

  TaskNode copyTaskWith({
    TaskEntity? task,
    int? indent,
    Map<String, dynamic>? metadata,
  }) {
    return TaskNode.fromTask(
      (task ?? this.task).copyWith(),
      indent: indent ?? this.indent,
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
    ).task.copyWith(name: (text ?? name).toPlainText());
    return TaskNode.fromTask(task, metadata: metadata ?? this.metadata);
  }

  @override
  bool hasEquivalentContent(DocumentNode other) {
    return other is TaskNode &&
        status == other.status &&
        text == other.text &&
        description == other.description;
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
}

extension TaskNodeType on DocumentNode {
  TaskNode get asTask => this as TaskNode;
}
