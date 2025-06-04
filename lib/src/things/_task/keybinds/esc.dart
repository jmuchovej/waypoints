import 'package:flutter/services.dart';

import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/things/_task/node.dart';

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
