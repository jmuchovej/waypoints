import 'package:flutter/material.dart';
import 'package:path/path.dart';
import '../editor.dart';
import '../../things/thing/interface.dart';
import '../../things/thing/thing.entity.dart';
import '../../things/task/task.entity.dart';
import '../../things/task/task_repository.dart';

const NamedAttribution taskAttribution = NamedAttribution("task");
const BlockSelector taskSelector = BlockSelector("task");
const NamedAttribution taskTitleAttribution = NamedAttribution("taskTitle");

/// Styles all task components to apply top padding
final taskStyles = StyleRule(taskSelector, (document, node) {
  if (node is! TaskNode) {
    return {};
  }

  // return {Styles.padding: const CascadingPadding.only(top: 6)};
  return {};
});

// Cache for indentation values to avoid repeated parent traversal
final _indentationCache = <int, int>{};

/// Clear the indentation cache (call when tasks are refreshed)
void clearIndentationCache() {
  _indentationCache.clear();
}

int _computeIndentation(TaskEntity? task, int indent) {
  if (indent != -1) return indent;
  if (task == null) return 0;

  // Check cache first
  final cached = _indentationCache[task.id];
  if (cached != null) return cached;

  // Compute indentation by traversing parents
  ThingEntity? ref = task.thing.target;
  while (ref != null &&
      ref.parent.target != null &&
      ref.type == TaskEntity.key) {
    indent += 1;
    ref = ref.parent.target;
  }

  // Cache the result
  _indentationCache[task.id] = indent;

  return indent;
}

@immutable
class TaskNode extends TextNode implements ThingNode {
  factory TaskNode.fromUUID(
    String uuid,
    TaskRepository repo, {
    Map<String, dynamic>? metadata,
    int indent = -1,
  }) {
    final task = repo.byUUID(uuid);
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
  DateTime get dueDate => task.dueDate;
  TaskStatus get status => task.status;
  TaskPriority get priority => task.priority;
  TaskTime get time => task.time;
  TaskEnergy get energy => task.energy;

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
