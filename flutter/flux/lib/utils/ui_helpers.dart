import 'package:flutter/material.dart';
import '../models/task.dart' as Task;
import 'package:iconify/icons/phosphor.dart';
import 'package:iconify/iconify.dart';

extension TaskStatusIcons on Task.Status {
  Widget get icon {
    return UIHelpers.buildStatusIcon(this);
  }
}

extension TaskPriorityIcons on Task.Priority {
  Widget get icon {
    return UIHelpers.buildPriorityIndicator(this);
  }
}

class UIHelpers {
  static Widget buildStatusIcon(Task.Status status) {
    String icon;
    Color color;

    switch (status) {
      case Task.Status.todo:
        icon = Phosphor["circle-bold"];
        color = Colors.grey;
        break;
      case Task.Status.started:
        icon = Phosphor["play-circle-bold"];
        color = Colors.blue;
        break;
      case Task.Status.waiting:
        icon = Phosphor["hourglass-bold"];
        color = Colors.orange;
        break;
      case Task.Status.paused:
        icon = Phosphor["pause-circle-bold"];
        color = Colors.deepPurple;
        break;
      case Task.Status.workInProgress:
        icon = Phosphor["traffic-cone-bold"];
        color = Colors.amber;
        break;
      case Task.Status.scheduled:
        icon = Phosphor["calendar-dot-bold"];
        color = Colors.teal;
        break;
      case Task.Status.stopped:
        icon = Phosphor["stop-circle-bold"];
        color = Colors.red;
        break;
      case Task.Status.dropped:
        icon = Phosphor["x-circle-bold"];
        color = Colors.red[900]!;
        break;
      case Task.Status.complete:
        icon = Phosphor["check-circle-bold"];
        color = Colors.green;
        break;
      case Task.Status.archived:
        icon = Phosphor["archive-box-fill"];
        color = Colors.grey[700]!;
        break;
      case Task.Status.deleted:
        icon = Phosphor["trash-bold"];
        color = Colors.red[900]!;
        break;
      case Task.Status.unknown:
        icon = Phosphor["seal-question-fill"];
        color = Colors.grey;
        break;
    }

    return Iconify(icon, color: color);
  }

  static Widget buildPriorityIndicator(Task.Priority? priority) {
    if (priority == null || priority == Task.Priority.unknown) {
      return const SizedBox.shrink();
    }

    Color color;
    String label;

    switch (priority) {
      case Task.Priority.L:
        color = Colors.green;
        label = 'Low';
        break;
      case Task.Priority.M:
        color = Colors.orange;
        label = 'Med';
        break;
      case Task.Priority.H:
        color = Colors.red;
        label = 'High';
        break;
      default:
        color = Colors.grey;
        label = '';
    }

    final icon = Iconify(Phosphor["exclamation-mark-fill"], color: color);
    return icon;

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.2),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontSize: 12,
          fontWeight: FontWeight.bold,
        ),
      ),
    );
  }

  static Color getEstimatedTimeColor(Task.EstimatedTime? time) {
    if (time == null) return Colors.grey;

    switch (time) {
      case Task.EstimatedTime.XS:
        return Colors.green[300]!;
      case Task.EstimatedTime.S:
        return Colors.green;
      case Task.EstimatedTime.M:
        return Colors.amber;
      case Task.EstimatedTime.L:
        return Colors.orange;
      case Task.EstimatedTime.XL:
        return Colors.red;
      default:
        return Colors.grey;
    }
  }

  static Color getEstimatedEnergyColor(Task.EstimatedEnergy? energy) {
    if (energy == null) return Colors.grey;

    switch (energy) {
      case Task.EstimatedEnergy.XS:
        return Colors.green[300]!;
      case Task.EstimatedEnergy.S:
        return Colors.green;
      case Task.EstimatedEnergy.M:
        return Colors.amber;
      case Task.EstimatedEnergy.L:
        return Colors.orange;
      case Task.EstimatedEnergy.XL:
        return Colors.red;
      default:
        return Colors.grey;
    }
  }
}
