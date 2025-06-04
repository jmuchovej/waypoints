import 'package:flutter/services.dart';

import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/things/_task/node.dart';

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
