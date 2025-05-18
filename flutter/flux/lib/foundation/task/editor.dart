import 'package:flux/foundation/task/_edits.dart';
import 'package:flux/editor.dart';

final List<EditRequestHandler> taskRequestHandlers = [
  // (editor, request) =>
  //     request is ConvertParagraphToTaskRequest
  //         ? ConvertParagraphToTaskCommand(
  //           nodeId: request.nodeId,
  //           status: request.status,
  //         )
  //         : null,
  // (editor, request) =>
  //     request is ConvertTaskToParagraphRequest
  //         ? ConvertTaskToParagraphCommand(
  //           nodeId: request.nodeId,
  //           paragraphMetadata: request.paragraphMetadata,
  //         )
  //         : null,
  // (editor, request) =>
  //     request is DeleteUpstreamAtBeginningOfNodeRequest &&
  //             request.node is TaskNode
  //         ? ConvertTaskToParagraphCommand(
  //           nodeId: request.node.id,
  //           paragraphMetadata: request.node.metadata,
  //         )
  //         : null,
  (editor, request) =>
      request is ChangeTaskStatusRequest
          ? ChangeTaskStatusCommand(
            nodeId: request.nodeId,
            status: request.status,
          )
          : null,
  (editor, request) =>
      request
              is IndentTaskRequest //
          ? IndentTaskCommand(request.nodeId)
          : null,
  (editor, request) =>
      request
              is UnIndentTaskRequest //
          ? UnIndentTaskCommand(request.nodeId)
          : null,
  (editor, request) =>
      request
              is SetTaskIndentRequest //
          ? SetTaskIndentCommand(request.nodeId, request.indent)
          : null,
  (editor, request) =>
      request is SplitExistingTaskRequest
          ? SplitExistingTaskCommand(
            nodeId: request.existingNodeId,
            splitOffset: request.splitOffset,
            newNodeId: request.newNodeId,
          )
          : null,
];

final List<EditReaction> taskReactionPipeline = [
  UpdateSubTaskIndentAfterTaskDeletionReaction(),
];
