import '../models/task.dart' as Task;

extension TaskStatusExtension on Task.Status {
  String toDisplayString() {
    switch (this) {
      case Task.Status.todo:
        return 'To Do';
      case Task.Status.started:
        return 'Started';
      case Task.Status.waiting:
        return 'Waiting';
      case Task.Status.paused:
        return 'Paused';
      case Task.Status.workInProgress:
        return 'Work in Progress';
      case Task.Status.scheduled:
        return 'Scheduled';
      case Task.Status.stopped:
        return 'Stopped';
      case Task.Status.dropped:
        return 'Dropped';
      case Task.Status.complete:
        return 'Complete';
      case Task.Status.archived:
        return 'Archived';
      case Task.Status.deleted:
        return 'Deleted';
      default:
        return toString().split('.').last;
    }
  }
}

extension PriorityExtension on Task.Priority {
  String toDisplayString() {
    switch (this) {
      case Task.Priority.unknown:
        return 'Unknown';
      case Task.Priority.L:
        return 'Low';
      case Task.Priority.M:
        return 'Medium';
      case Task.Priority.H:
        return 'High';
      default:
        return toString().split('.').last;
    }
  }
}

extension EstimatedTimeExtension on Task.EstimatedTime {
  String toDisplayString() {
    switch (this) {
      case Task.EstimatedTime.unknown:
        return 'Unknown';
      case Task.EstimatedTime.XS:
        return 'XS (Extra Small)';
      case Task.EstimatedTime.S:
        return 'S (Small)';
      case Task.EstimatedTime.M:
        return 'M (Medium)';
      case Task.EstimatedTime.L:
        return 'L (Large)';
      case Task.EstimatedTime.XL:
        return 'XL (Extra Large)';
      default:
        return toString().split('.').last;
    }
  }
}

extension EstimatedEnergyExtension on Task.EstimatedEnergy {
  String toDisplayString() {
    switch (this) {
      case Task.EstimatedEnergy.unknown:
        return 'Unknown';
      case Task.EstimatedEnergy.XS:
        return 'XS (Extra Small)';
      case Task.EstimatedEnergy.S:
        return 'S (Small)';
      case Task.EstimatedEnergy.M:
        return 'M (Medium)';
      case Task.EstimatedEnergy.L:
        return 'L (Large)';
      case Task.EstimatedEnergy.XL:
        return 'XL (Extra Large)';
      default:
        return toString().split('.').last;
    }
  }
}

extension StringExtension on String {
  String capitalize() {
    if (isEmpty) return this;
    return "${this[0].toUpperCase()}${substring(1)}";
  }
}
