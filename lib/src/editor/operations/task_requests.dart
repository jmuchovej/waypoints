import '../super_editor.dart';

/// Request to toggle a task's completion status
class ToggleTaskStatusRequest implements EditRequest {
  const ToggleTaskStatusRequest({required this.nodeId});

  /// The ID of the TaskNode to toggle
  final String nodeId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ToggleTaskStatusRequest &&
          runtimeType == other.runtimeType &&
          nodeId == other.nodeId;

  @override
  int get hashCode => nodeId.hashCode;

  @override
  String toString() => 'ToggleTaskStatusRequest(nodeId: $nodeId)';
}
