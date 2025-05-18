import 'package:flux/editor.dart';
import 'package:flux/foundation/task.dart';

final List<EditRequestHandler> newlineRequestHandlers = [
  (editor, request) =>
      request
              is InsertSoftNewlineAtCaretRequest //
          ? const InsertSoftNewlineCommand()
          : null,
  (editor, request) {
    if (request is! InsertNewlineAtCaretRequest) {
      return null;
    }

    final selection = editor.composer.selection;
    if (selection == null) {
      return null;
    }

    final base = selection.base;
    if (editor.document.getNodeById(base.nodeId) is! ListItemNode) {
      return null;
    }

    return InsertNewlineInListItemAtCaretCommand(request.newNodeId);
  },
  (editor, request) {
    if (request is! InsertNewlineAtCaretRequest) {
      return null;
    }

    final selection = editor.composer.selection;
    if (selection == null) {
      return null;
    }

    final base = selection.base;
    final node = editor.document.getNodeById(base.nodeId);
    if (node is! ParagraphNode) {
      return null;
    }
    if (node.metadata[NodeMetadata.blockType] != codeAttribution) {
      return null;
    }

    return InsertNewlineInCodeBlockAtCaretCommand(request.newNodeId);
  },
  (editor, request) {
    if (request is! InsertNewlineAtCaretRequest) {
      return null;
    }

    final selection = editor.composer.selection;
    if (selection == null) {
      return null;
    }

    final base = selection.base;
    if (editor.document.getNodeById(base.nodeId) is! TaskNode) {
      return null;
    }

    return InsertNewlineInTaskAtCaretCommand(request.newNodeId);
  },
  (editor, request) =>
      request
              is InsertNewlineAtCaretRequest //
          ? DefaultInsertNewlineAtCaretCommand(request.newNodeId)
          : null,
];
