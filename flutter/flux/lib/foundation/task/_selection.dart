import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flux/foundation/task.dart';
import 'package:flux/editor.dart';

class TaskSelectionNotifier extends StateNotifier<Set<String>> {
  TaskSelectionNotifier() : super({});

  bool isSelected(String taskID) => state.contains(taskID);

  void select(String taskID) {
    state = {...state, taskID};
  }

  void deselect(String taskID) {
    state = {...state}..remove(taskID);
  }

  void toggle(String taskID) {
    if (isSelected(taskID)) {
      deselect(taskID);
    } else {
      select(taskID);
    }
  }

  void clear() {
    state = {};
  }
}

final taskSelectionProvider =
    StateNotifierProvider<TaskSelectionNotifier, Set<String>>(
      (ref) => TaskSelectionNotifier(),
    );

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

  editContext.editor.execute([DeselectTaskRequest(node.id)]);

  return ExecutionInstruction.haltExecution;
}

class DeselectTaskRequest extends EditRequest {
  final String taskID;

  DeselectTaskRequest(this.taskID);
}
