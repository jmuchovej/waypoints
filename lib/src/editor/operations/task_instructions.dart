import 'package:flutter/services.dart';

import '../super_editor.dart';
import '../nodes/task_node.dart';

/// region: Esc ////////////////////////////////////////////////////////////////
ExecutionInstruction escToDeselectTask({
  required SuperEditorContext editContext,
  required KeyEvent keyEvent,
}) {
  if (keyEvent is! KeyDownEvent && keyEvent is! KeyRepeatEvent) {
    return ExecutionInstruction.continueExecution;
  }

  if (keyEvent.logicalKey != LogicalKeyboardKey.escape) {
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

  // editContext.editor.execute([DeselectTaskRequest(node.id)]);

  return ExecutionInstruction.haltExecution;
}

class TaskSelectionRequest extends EditRequest {
  TaskSelectionRequest(this.taskID);

  final String taskID;
}

class TaskSelectionCommand extends EditCommand {
  TaskSelectionCommand(this.taskID);

  final String taskID;

  @override
  void execute(EditContext context, CommandExecutor executor) {
    final document = context.document;

    final task = document.getNodeById(taskID);
    if (task is! TaskNode) {
      // The specified node isn't a task. Nothing for us to indent.
      return;
    }

    document.replaceNodeById(task.id, task.copyTaskWith());

    executor.logChanges([DocumentEdit(NodeChangeEvent(task.id))]);
  }
}

/// endregion //////////////////////////////////////////////////////////////////

/// region: Spacebar ///////////////////////////////////////////////////////////
ExecutionInstruction spacebarToggleTaskEditMode({
  required SuperEditorContext editorContext,
  required KeyEvent keyEvent,
}) {
  if (keyEvent is! KeyDownEvent ||
      keyEvent.logicalKey != LogicalKeyboardKey.space) {
    return ExecutionInstruction.continueExecution;
  }

  final selection = editorContext.composer.selection;
  if (selection == null) {
    return ExecutionInstruction.continueExecution;
  }

  final document = editorContext.document;
  final node = document.getNodeById(selection.extent.nodeId);
  if (node == null || node is! TaskNode || node.asTask.inEditMode) {
    return ExecutionInstruction.continueExecution;
  }

  editorContext.editor.execute([ToggleTaskEditModeRequest(node.id)]);

  return ExecutionInstruction.haltExecution;
}

class ToggleTaskEditModeRequest extends EditRequest {
  ToggleTaskEditModeRequest(this.taskID);
  final String taskID;
}

class ToggleTaskEditModeCommand extends EditCommand {
  ToggleTaskEditModeCommand(this.taskID);
  final String taskID;

  @override
  void execute(EditContext context, CommandExecutor executor) {
    final document = context.document;
    final node = document.getNodeById(taskID) as TaskNode;

    final task = node.copyTaskWith(inEditMode: !node.inEditMode);
    document.replaceNodeById(taskID, task);

    executor.logChanges([DocumentEdit(NodeChangeEvent(taskID))]);
  }
}

/// endregion //////////////////////////////////////////////////////////////////
