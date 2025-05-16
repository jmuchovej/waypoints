import 'package:flutter/services.dart';
import 'package:flux/foundation/task.dart';
import 'package:flux/ui/editor.dart';

ExecutionInstruction enterToInsertNewTask({
  required SuperEditorContext editContext,
  required KeyEvent keyEvent,
}) {
  if (keyEvent is! KeyDownEvent && keyEvent is! KeyRepeatEvent) {
    return ExecutionInstruction.continueExecution;
  }

  // We only care about ENTER.
  if (keyEvent.logicalKey != LogicalKeyboardKey.enter &&
      keyEvent.logicalKey != LogicalKeyboardKey.numpadEnter) {
    return ExecutionInstruction.continueExecution;
  }

  // We only care when the selection is collapsed to a caret.
  final selection = editContext.composer.selection;
  if (selection == null || !selection.isCollapsed) {
    return ExecutionInstruction.continueExecution;
  }

  // We only care about TaskNodes.
  final node = editContext.document.getNodeById(selection.extent.nodeId);
  if (node is! TaskNode) {
    return ExecutionInstruction.continueExecution;
  }

  editContext.editor.execute([
    InsertNewlineAtCaretRequest(Editor.createNodeId()),
  ]);

  return ExecutionInstruction.haltExecution;
}

ExecutionInstruction backspaceToConvertTaskToParagraph({
  required SuperEditorContext editContext,
  required KeyEvent keyEvent,
}) {
  if (keyEvent is! KeyDownEvent && keyEvent is! KeyRepeatEvent) {
    return ExecutionInstruction.continueExecution;
  }

  if (keyEvent.logicalKey != LogicalKeyboardKey.backspace) {
    return ExecutionInstruction.continueExecution;
  }

  if (editContext.composer.selection == null) {
    return ExecutionInstruction.continueExecution;
  }
  if (!editContext.composer.selection!.isCollapsed) {
    return ExecutionInstruction.continueExecution;
  }

  final node = editContext.document.getNodeById(
    editContext.composer.selection!.extent.nodeId,
  );
  if (node is! TaskNode) {
    return ExecutionInstruction.continueExecution;
  }

  if ((editContext.composer.selection!.extent.nodePosition as TextPosition)
          .offset >
      0) {
    // The selection isn't at the beginning.
    return ExecutionInstruction.continueExecution;
  }

  editContext.editor.execute([DeleteUpstreamAtBeginningOfNodeRequest(node)]);

  return ExecutionInstruction.haltExecution;
}

ExecutionInstruction tabToIndentTask({
  required SuperEditorContext editContext,
  required KeyEvent keyEvent,
}) {
  if (keyEvent is! KeyDownEvent && keyEvent is! KeyRepeatEvent) {
    return ExecutionInstruction.continueExecution;
  }

  if (keyEvent.logicalKey != LogicalKeyboardKey.tab) {
    return ExecutionInstruction.continueExecution;
  }

  if (HardwareKeyboard.instance.isShiftPressed) {
    // Don't indent if Shift is pressed - that's for un-indenting.
    return ExecutionInstruction.continueExecution;
  }

  final selection = editContext.composer.selection;
  if (selection == null) {
    return ExecutionInstruction.continueExecution;
  }

  if (selection.base.nodeId != selection.extent.nodeId) {
    // Selection spans nodes, so even if this selection includes a task,
    // it includes other stuff, too. So we can't treat this as a task indentation.
    return ExecutionInstruction.continueExecution;
  }

  final node = editContext.document.getNodeById(
    editContext.composer.selection!.extent.nodeId,
  );
  if (node is! TaskNode) {
    return ExecutionInstruction.continueExecution;
  }

  final taskAbove = editContext.document.getNodeBefore(node);
  if (taskAbove == null) {
    // No task above us, so we can't indent.
    return ExecutionInstruction.continueExecution;
  }
  if (taskAbove is! TaskNode) {
    // The node above isn't a task. We can't indent.
    return ExecutionInstruction.continueExecution;
  }

  final maxIndent = taskAbove.indent + 1;
  if (node.indent >= maxIndent) {
    // Can't indent any further.
    return ExecutionInstruction.continueExecution;
  }

  editContext.editor.execute([IndentTaskRequest(node.id)]);

  return ExecutionInstruction.haltExecution;
}

ExecutionInstruction shiftTabToUnIndentTask({
  required SuperEditorContext editContext,
  required KeyEvent keyEvent,
}) {
  if (keyEvent is! KeyDownEvent && keyEvent is! KeyRepeatEvent) {
    return ExecutionInstruction.continueExecution;
  }

  if (keyEvent.logicalKey != LogicalKeyboardKey.tab) {
    return ExecutionInstruction.continueExecution;
  }
  if (!HardwareKeyboard.instance.isShiftPressed) {
    return ExecutionInstruction.continueExecution;
  }

  final selection = editContext.composer.selection;
  if (selection == null) {
    return ExecutionInstruction.continueExecution;
  }

  if (selection.base.nodeId != selection.extent.nodeId) {
    // Selection spans nodes, so even if this selection includes a task,
    // it includes other stuff, too. So we can't treat this as a task indentation.
    return ExecutionInstruction.continueExecution;
  }

  final node = editContext.document.getNodeById(
    editContext.composer.selection!.extent.nodeId,
  );
  if (node is! TaskNode) {
    return ExecutionInstruction.continueExecution;
  }

  if (node.indent == 0) {
    // Can't un-indent any further.
    return ExecutionInstruction.continueExecution;
  }

  editContext.editor.execute([UnIndentTaskRequest(node.id)]);

  return ExecutionInstruction.haltExecution;
}

ExecutionInstruction backspaceToUnIndentTask({
  required SuperEditorContext editContext,
  required KeyEvent keyEvent,
}) {
  if (keyEvent is! KeyDownEvent && keyEvent is! KeyRepeatEvent) {
    return ExecutionInstruction.continueExecution;
  }

  if (keyEvent.logicalKey != LogicalKeyboardKey.backspace) {
    return ExecutionInstruction.continueExecution;
  }

  final selection = editContext.composer.selection;
  if (selection == null) {
    return ExecutionInstruction.continueExecution;
  }

  if (selection.base.nodeId != selection.extent.nodeId) {
    // Selection spans nodes, so even if this selection includes a task,
    // it includes other stuff, too. So we can't treat this as a task indentation.
    return ExecutionInstruction.continueExecution;
  }

  final node = editContext.document.getNodeById(
    editContext.composer.selection!.extent.nodeId,
  );
  if (node is! TaskNode) {
    return ExecutionInstruction.continueExecution;
  }
  if ((editContext.composer.selection!.extent.nodePosition as TextPosition)
          .offset >
      0) {
    // Backspace should only un-indent if the caret is at the start of the text.
    return ExecutionInstruction.continueExecution;
  }

  if (node.indent == 0) {
    // Can't un-indent any further.
    return ExecutionInstruction.continueExecution;
  }

  editContext.editor.execute([UnIndentTaskRequest(node.id)]);

  return ExecutionInstruction.haltExecution;
}

/// An [EditCommand] that inserts a newline when the caret sits within a [TaskNode].
///
/// This command adds the following behaviors beyond the usual:
///  * When the caret is in the middle of a task, splits the task into two tasks.
///
///  * When the caret is at the end of a task, inserts a new empty task, instead of an
///    empty paragraph.
///
///  * Inserting a newline into an empty task converts it into a paragraph instead of
///    inserting a new task.
class InsertNewlineInTaskAtCaretCommand
    extends BaseInsertNewlineAtCaretCommand {
  const InsertNewlineInTaskAtCaretCommand(this.newNodeId);

  /// {@macro newNodeId}
  final String newNodeId;

  @override
  void doInsertNewline(
    EditContext context,
    CommandExecutor executor,
    DocumentPosition caretPosition,
    NodePosition caretNodePosition,
  ) {
    final node = context.document.getNodeById(caretPosition.nodeId);
    if (caretNodePosition is! TextNodePosition || node is! TaskNode) {
      // We don't know how to deal with this kind of node.
      return;
    }

    if (node.text.isEmpty) {
      // The task is empty. Convert it to a paragraph.
      executor.executeCommand(ConvertTaskToParagraphCommand(nodeId: node.id));
      return;
    }

    executor
      ..executeCommand(
        SplitExistingTaskCommand(
          nodeId: node.id,
          splitOffset: caretNodePosition.offset,
          newNodeId: newNodeId,
        ),
      )
      ..executeCommand(
        ChangeSelectionCommand(
          DocumentSelection.collapsed(
            position: DocumentPosition(
              nodeId: newNodeId,
              nodePosition: const TextNodePosition(offset: 0),
            ),
          ),
          SelectionChangeType.insertContent,
          SelectionReason.userInteraction,
        ),
      );
  }
}

class ChangeTaskCompletionRequest implements EditRequest {
  ChangeTaskCompletionRequest({required this.nodeId, required this.status});

  final String nodeId;
  final TaskStatus status;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ChangeTaskCompletionRequest &&
          runtimeType == other.runtimeType &&
          nodeId == other.nodeId &&
          status == other.status;

  @override
  int get hashCode => nodeId.hashCode ^ status.hashCode;
}

class ChangeTaskCompletionCommand extends EditCommand {
  ChangeTaskCompletionCommand({required this.nodeId, required this.status});

  final String nodeId;
  final TaskStatus status;

  @override
  HistoryBehavior get historyBehavior => HistoryBehavior.undoable;

  @override
  void execute(EditContext context, CommandExecutor executor) {
    final taskNode = context.document.getNodeById(nodeId);
    if (taskNode is! TaskNode) {
      return;
    }

    context.document.replaceNodeById(
      taskNode.id,
      taskNode.copyTaskWith(task: taskNode.task.copyWith(status: status)),
    );

    executor.logChanges([DocumentEdit(NodeChangeEvent(nodeId))]);
  }
}

class ConvertParagraphToTaskRequest implements EditRequest {
  const ConvertParagraphToTaskRequest({
    required this.nodeId,
    this.status = TaskStatus.todo,
  });

  final String nodeId;
  final TaskStatus status;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ConvertParagraphToTaskRequest &&
          runtimeType == other.runtimeType &&
          nodeId == other.nodeId &&
          status == other.status;

  @override
  int get hashCode => nodeId.hashCode ^ status.hashCode;
}

class ConvertParagraphToTaskCommand extends EditCommand {
  const ConvertParagraphToTaskCommand({
    required this.nodeId,
    this.status = TaskStatus.todo,
  });

  final String nodeId;
  final TaskStatus status;

  @override
  HistoryBehavior get historyBehavior => HistoryBehavior.undoable;

  @override
  void execute(EditContext context, CommandExecutor executor) {
    final document = context.document;
    final existingNode = document.getNodeById(nodeId);
    if (existingNode is! ParagraphNode) {
      editorOpsLog.warning(
        "Tried to convert ParagraphNode with ID '$nodeId' to TaskNode, but that node has the wrong type: ${existingNode.runtimeType}",
      );
      return;
    }

    final taskNode = TaskNode.fromTask(
      TaskEntity(
        uuid: existingNode.id,
        name: existingNode.text.toPlainText(),
        status: status,
      ),
    );

    executor.executeCommand(
      ReplaceNodeCommand(existingNodeId: existingNode.id, newNode: taskNode),
    );
  }
}

class ConvertTaskToParagraphRequest implements EditRequest {
  const ConvertTaskToParagraphRequest({
    required this.nodeId,
    this.paragraphMetadata,
  });

  final String nodeId;
  final Map<String, dynamic>? paragraphMetadata;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ConvertTaskToParagraphRequest &&
          runtimeType == other.runtimeType &&
          nodeId == other.nodeId &&
          paragraphMetadata == other.paragraphMetadata;

  @override
  int get hashCode => nodeId.hashCode ^ paragraphMetadata.hashCode;
}

class ConvertTaskToParagraphCommand extends EditCommand {
  const ConvertTaskToParagraphCommand({
    required this.nodeId,
    this.paragraphMetadata,
  });

  final String nodeId;
  final Map<String, dynamic>? paragraphMetadata;

  @override
  HistoryBehavior get historyBehavior => HistoryBehavior.undoable;

  @override
  void execute(EditContext context, CommandExecutor executor) {
    final document = context.document;
    final node = document.getNodeById(nodeId);
    final taskNode = node as TaskNode;
    final newMetadata = Map<String, dynamic>.from(paragraphMetadata ?? {});
    newMetadata["blockType"] = paragraphAttribution;

    final newParagraphNode = ParagraphNode(
      id: taskNode.id,
      text: taskNode.text,
      metadata: newMetadata,
    );
    document.replaceNodeById(taskNode.id, newParagraphNode);

    executor.logChanges([DocumentEdit(NodeChangeEvent(taskNode.id))]);
  }
}

class SplitExistingTaskRequest implements EditRequest {
  const SplitExistingTaskRequest({
    required this.existingNodeId,
    required this.splitOffset,
    this.newNodeId,
  });

  final String existingNodeId;
  final int splitOffset;
  final String? newNodeId;
}

class SplitExistingTaskCommand extends EditCommand {
  const SplitExistingTaskCommand({
    required this.nodeId,
    required this.splitOffset,
    this.newNodeId,
  });

  final String nodeId;
  final int splitOffset;
  final String? newNodeId;

  @override
  HistoryBehavior get historyBehavior => HistoryBehavior.undoable;

  @override
  void execute(EditContext editContext, CommandExecutor executor) {
    final document = editContext.document;
    final composer = editContext.find<MutableDocumentComposer>(
      Editor.composerKey,
    );
    final selection = composer.selection;

    // We only care when the caret sits at the end of a TaskNode.
    if (selection == null || !selection.isCollapsed) {
      return;
    }

    // We only care about TaskNodes.
    final node = document.getNodeById(selection.extent.nodeId);
    if (node is! TaskNode) {
      return;
    }

    // Ensure the split offset is valid.
    if (splitOffset < 0 || splitOffset > node.text.length + 1) {
      return;
    }

    final now = DateTime.now();
    final newTask = node.task.copyWith(
      uuid: newNodeId ?? Editor.createNodeId(),

      /// TODO refactor [TaskEntity] to store [AttributedText]!
      name: node.name.copyText(splitOffset).toPlainText(),
      createDate: now,
      modifyDate: now,
      children: [],
    );
    final newTaskNode = TaskNode.fromTask(newTask);

    // Remove the text after the caret from the currently selected TaskNode.
    final updatedNode = node.copyTaskWith(
      /// TODO refactor [TaskEntity] to store [AttributedText]!
      task: node.task.copyWith(
        name:
            node.name
                .removeRegion(
                  startOffset: splitOffset,
                  endOffset: node.name.length,
                )
                .toPlainText(),
      ),
    );
    document.replaceNodeById(node.id, updatedNode);

    // Insert a new TextNode after the currently selected TaskNode.
    document.insertNodeAfter(
      existingNodeId: updatedNode.id,
      newNode: newTaskNode,
    );

    // Move the caret to the beginning of the new TaskNode.
    final oldSelection = composer.selection;
    final oldComposingRegion = composer.composingRegion.value;
    final newSelection = DocumentSelection.collapsed(
      position: DocumentPosition(
        nodeId: newTaskNode.id,
        nodePosition: const TextNodePosition(offset: 0),
      ),
    );

    composer.setSelectionWithReason(
      newSelection,
      SelectionReason.userInteraction,
    );
    composer.setComposingRegion(null);

    executor.logChanges([
      SplitTaskIntention.start(),
      DocumentEdit(NodeChangeEvent(node.id)),
      DocumentEdit(
        NodeInsertedEvent(
          newTaskNode.id,
          document.getNodeIndexById(newTaskNode.id),
        ),
      ),
      SelectionChangeEvent(
        oldSelection: oldSelection,
        newSelection: newSelection,
        changeType: SelectionChangeType.pushCaret,
        reason: SelectionReason.userInteraction,
      ),
      ComposingRegionChangeEvent(
        oldComposingRegion: oldComposingRegion,
        newComposingRegion: null,
      ),
      SplitTaskIntention.end(),
    ]);
  }
}

class SplitTaskIntention extends Intention {
  SplitTaskIntention.start() : super.start();

  SplitTaskIntention.end() : super.end();
}

class IndentTaskRequest implements EditRequest {
  const IndentTaskRequest(this.nodeId);

  final String nodeId;
}

class IndentTaskCommand extends EditCommand {
  const IndentTaskCommand(this.nodeId);

  final String nodeId;

  @override
  void execute(EditContext context, CommandExecutor executor) {
    final document = context.document;

    final task = document.getNodeById(nodeId);
    if (task is! TaskNode) {
      // The specified node isn't a task. Nothing for us to indent.
      return;
    }

    final taskAbove = document.getNodeBefore(task);
    if (taskAbove is! TaskNode) {
      // There's no task above this task, therefore it can't be indented.
      return;
    }

    final maxIndent = taskAbove.indent + 1;
    if (task.indent >= maxIndent) {
      // This task is already at max indentation.
      return;
    }

    // Increase the task indentation.
    document.replaceNodeById(
      task.id,
      task.copyTaskWith(indent: task.indent + 1),
    );

    executor.logChanges([DocumentEdit(NodeChangeEvent(task.id))]);
  }
}

class UnIndentTaskRequest implements EditRequest {
  const UnIndentTaskRequest(this.nodeId);

  final String nodeId;
}

class UnIndentTaskCommand extends EditCommand {
  const UnIndentTaskCommand(this.nodeId);

  final String nodeId;

  @override
  void execute(EditContext context, CommandExecutor executor) {
    final document = context.document;

    final task = document.getNodeById(nodeId);
    if (task is! TaskNode) {
      // The specified node isn't a task. Nothing for us to indent.
      return;
    }

    if (task.indent == 0) {
      // This task is already at minimum indent. Nothing to do.
      return;
    }

    final subTasks = <TaskNode>[];
    var nextNode = document.getNodeAfter(task);
    while (nextNode != null) {
      final subTask = nextNode;
      if (subTask is! TaskNode) {
        break;
      }
      if (subTask.indent <= task.indent) {
        break;
      }

      subTasks.add(subTask);
      nextNode = document.getNodeAfter(nextNode);
    }

    final changeLog = <DocumentEdit>[];

    // Decrease the task indentation of the desired task.
    document.replaceNodeById(
      task.id,
      task.copyTaskWith(indent: task.indent - 1),
    );

    changeLog.add(DocumentEdit(NodeChangeEvent(task.id)));

    // Decrease the indentation of the sub-tasks.
    for (final subTask in subTasks) {
      document.replaceNodeById(
        subTask.id,
        subTask.copyTaskWith(indent: subTask.indent - 1),
      );

      changeLog.add(DocumentEdit(NodeChangeEvent(subTask.id)));
    }

    // Log all changes.
    executor.logChanges(changeLog);
  }
}

/// Sets the indent of the task with ID [nodeId] to the given [indent].
///
/// This request doesn't verify any rules about allowed indentation
/// levels. It blindly applies the indent. Therefore, this request should
/// only be issued from places that have already validated the result.
class SetTaskIndentRequest implements EditRequest {
  const SetTaskIndentRequest(this.nodeId, this.indent);

  final String nodeId;
  final int indent;
}

class SetTaskIndentCommand extends EditCommand {
  const SetTaskIndentCommand(this.nodeId, this.indent);

  final String nodeId;
  final int indent;

  @override
  void execute(EditContext context, CommandExecutor executor) {
    final document = context.document;

    final task = document.getNodeById(nodeId);
    if (task is! TaskNode) {
      // The specified node isn't a task. Nothing for us to indent.
      return;
    }

    document.replaceNodeById(task.id, task.copyTaskWith(indent: indent));

    executor.logChanges([DocumentEdit(NodeChangeEvent(task.id))]);
  }
}

class UpdateSubTaskIndentAfterTaskDeletionReaction extends EditReaction {
  @override
  void modifyContent(
    EditContext editorContext,
    RequestDispatcher requestDispatcher,
    List<EditEvent> changeList,
  ) {
    final didDeleteTask =
        changeList
            .whereType<DocumentEdit>()
            .where(
              (edit) =>
                  edit.change is NodeRemovedEvent &&
                  (edit.change as NodeRemovedEvent).removedNode is TaskNode,
            )
            .isNotEmpty;
    if (!didDeleteTask) {
      // No tasks were deleted, so there are no task indentations to fix.
      return;
    }

    // At least one task was deleted. We're not sure where in the document the
    // tasks were before being deleted. Therefore, we check and fix every task
    // indentation in the document.
    final document = editorContext.document;
    final changeIndentationRequests = <EditRequest>[];
    int maxIndentation = 0;
    for (final node in document) {
      if (node is! TaskNode) {
        // This node isn't a task. The first task in a list of tasks
        // can't have an indent, so reset the max indent back to zero.
        maxIndentation = 0;
        continue;
      }

      if (node.indent > maxIndentation) {
        // This task has an indent that's too deep. Fix it by
        // settings its indent to the max allowed.
        changeIndentationRequests.add(
          SetTaskIndentRequest(node.id, maxIndentation),
        );

        // A task that follows this one is allowed (up to) the previous
        // max + 1.
        maxIndentation += 1;
        continue;
      }

      // This is a task with a legitimate indent. Update the
      // max indent tracker based on this task's level.
      maxIndentation = node.indent + 1;
    }

    if (changeIndentationRequests.isEmpty) {
      // No changes needed.
      return;
    }

    // Adjust all tasks with illegal indentations.
    requestDispatcher.execute(changeIndentationRequests);
  }
}
