import 'package:flutter/services.dart';
import 'package:tempo/core/editor/super_editor.dart';
import 'package:tempo/infrastructure/_logging.dart';
import 'package:tempo/core/editor/plugins/motion/motion.dart';

ExecutionInstruction escToReturnToNormalMode({
  required SuperEditorContext editContext,
  required KeyEvent keyEvent,
}) {
  pluginMotionLog.finest(
    'calling `escToReturnToNormalMode(${keyEvent.logicalKey})`',
  );
  if (keyEvent is! KeyDownEvent && keyEvent is! KeyRepeatEvent) {
    return ExecutionInstruction.continueExecution;
  }

  if (keyEvent.logicalKey != LogicalKeyboardKey.escape) {
    return ExecutionInstruction.continueExecution;
  }

  final context = editContext.editor.context;
  final mode = context.find<Motion>(Motion.key);
  if (mode.value != MotionMode.insert) {
    pluginMotionLog.finest('  not in INSERT mode!');
    return ExecutionInstruction.continueExecution;
  }

  mode.activateNormal();
  pluginMotionLog.finest('  activating NORMAL');
  pluginMotionLog.finest('    active mode=${mode.value.toDisplayString()}...');
  return ExecutionInstruction.haltExecution;
}

ExecutionInstruction iToEnterInsertMode({
  required SuperEditorContext editContext,
  required KeyEvent keyEvent,
}) {
  pluginMotionLog.finest(
    'calling `iToEnterInsertMode(${keyEvent.logicalKey})`',
  );
  if (keyEvent is! KeyDownEvent && keyEvent is! KeyRepeatEvent) {
    return ExecutionInstruction.continueExecution;
  }

  if (keyEvent.logicalKey != LogicalKeyboardKey.keyI) {
    return ExecutionInstruction.continueExecution;
  }

  final context = editContext.editor.context;
  final mode = context.find<Motion>(Motion.key);
  if (mode.value == MotionMode.insert) {
    pluginMotionLog.finest('  already in INSERT');
    return ExecutionInstruction.continueExecution;
  }

  mode.activateInsert();
  pluginMotionLog.finest('  activating INSERT');
  pluginMotionLog.finest('    active mode=${mode.value.toDisplayString()}...');
  return ExecutionInstruction.haltExecution;
}

ExecutionInstruction normalModeMoveN({
  required SuperEditorContext editContext,
  required KeyEvent keyEvent,
}) {
  pluginMotionLog.finest('calling `normalModeMoveN(${keyEvent.logicalKey})`');
  final acceptedKeys = [LogicalKeyboardKey.keyK, LogicalKeyboardKey.arrowUp];
  if (keyEvent is! KeyDownEvent && keyEvent is! KeyRepeatEvent) {
    return ExecutionInstruction.continueExecution;
  }

  if (!acceptedKeys.contains(keyEvent.logicalKey)) {
    return ExecutionInstruction.continueExecution;
  }

  final context = editContext.editor.context;
  final mode = context.find<Motion>(Motion.key);
  if (mode.value != MotionMode.normal) {
    pluginMotionLog.finest(
      '  not in ${MotionMode.normal.toDisplayString()}, ignoring!',
    );
    return ExecutionInstruction.continueExecution;
  }

  final nodeID = _moveSelectionToAdjacentNode(editContext, MotionDirection.N);
  pluginMotionLog.finest('  attempting to move to nodeID=${nodeID}');
  return ExecutionInstruction.haltExecution;
}

ExecutionInstruction normalModeMoveS({
  required SuperEditorContext editContext,
  required KeyEvent keyEvent,
}) {
  pluginMotionLog.finest('calling `normalModeMoveS(${keyEvent.logicalKey})`');
  final acceptedKeys = [LogicalKeyboardKey.keyJ, LogicalKeyboardKey.arrowDown];
  if (keyEvent is! KeyDownEvent && keyEvent is! KeyRepeatEvent) {
    return ExecutionInstruction.continueExecution;
  }

  if (!acceptedKeys.contains(keyEvent.logicalKey)) {
    return ExecutionInstruction.continueExecution;
  }

  final context = editContext.editor.context;
  final mode = context.find<Motion>(Motion.key);
  if (mode.value != MotionMode.normal) {
    pluginMotionLog.finest(
      '  not in ${MotionMode.normal.toDisplayString()}, ignoring!',
    );
    return ExecutionInstruction.continueExecution;
  }

  final nodeID = _moveSelectionToAdjacentNode(editContext, MotionDirection.S);
  pluginMotionLog.finest('  attempting to move to nodeID=${nodeID}');
  return ExecutionInstruction.haltExecution;
}

String? _moveSelectionToAdjacentNode(
  SuperEditorContext editContext,
  MotionDirection motion,
) {
  final selection = editContext.composer.selection;
  final document = editContext.document;
  DocumentNode? node;

  switch (motion) {
    case MotionDirection.N:
      node =
          selection != null
              ? document.getNodeBeforeById(selection.extent.nodeId)
              : document.last;
      node = node ?? document.last;
      break;
    case MotionDirection.S:
      node =
          selection != null
              ? document.getNodeAfterById(selection.extent.nodeId)
              : document.first;
      node = node ?? document.first;
      break;
    default:
      return null;
  }

  final base = DocumentPosition(
    nodeId: node.id,
    nodePosition: const TextNodePosition(offset: 0),
  );
  final extent = DocumentPosition(
    nodeId: node.id,
    nodePosition: const TextNodePosition(offset: 0),
  );
  editContext.editor.execute([
    ChangeSelectionRequest(
      DocumentSelection(base: base, extent: extent),
      SelectionChangeType.placeCaret,
      SelectionReason.userInteraction,
    ),
  ]);

  final component = editContext.documentLayout.getComponentByNodeId(node.id);
  if (component == null) {
    return null;
  }

  final componentRect = editContext.documentLayout.getRectForPosition(base);
  if (componentRect == null) {
    return null;
  }

  final scroller = editContext.scroller;
  final viewportHeight = scroller.viewportDimension;
  final currentOffset = scroller.scrollOffset;

  if (componentRect.top < currentOffset) {
    scroller.jumpTo(componentRect.top);
  } else if (componentRect.bottom > currentOffset + viewportHeight) {
    scroller.jumpBy(componentRect.bottom - viewportHeight);
  }

  return node.id;
}

ExecutionInstruction blockCharacterInsertionInNormalMode({
  required SuperEditorContext editContext,
  required KeyEvent keyEvent,
}) {
  if (keyEvent is! KeyDownEvent && keyEvent is! KeyRepeatEvent) {
    return ExecutionInstruction.continueExecution;
  }

  final context = editContext.editor.context;
  final mode = context.find<Motion>(Motion.key);
  pluginMotionLog.finest('received ${keyEvent.logicalKey}!');
  if (mode.value != MotionMode.insert) {
    pluginMotionLog.finest(
      '  not in ${MotionMode.insert.toDisplayString()}, ignoring!',
    );
    return ExecutionInstruction.haltExecution;
  }

  return ExecutionInstruction.continueExecution;
}
