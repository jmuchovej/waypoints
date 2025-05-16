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
      // executor.executeCommand(ConvertTaskToParagraphCommand(nodeId: node.id));
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
