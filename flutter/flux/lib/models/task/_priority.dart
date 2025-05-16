import 'package:flutter/material.dart';
import 'package:iconify/iconify.dart';

enum TaskPriority {
  L, // Low priority
  M, // Medium priority
  H, // High priority
}

void assertStableEnumValues() {
  assert(0 == TaskPriority.L.index);
  assert(1 == TaskPriority.M.index);
  assert(2 == TaskPriority.H.index);
}

extension PriorityColors on TaskPriority {
  Color get color {
    switch (this) {
      case TaskPriority.L:
        return Colors.green;
      case TaskPriority.M:
        return Colors.orange;
      case TaskPriority.H:
        return Colors.red;
    }
  }
}

extension PriorityIcons on TaskPriority {
  Widget get icon {
    return Icon(Icons.flag_circle_rounded, color: color);
  }
}

extension PriortyDisplayString on TaskPriority {
  String toDisplayString() {
    switch (this) {
      case TaskPriority.L:
        return 'Low';
      case TaskPriority.M:
        return 'Medium';
      case TaskPriority.H:
        return 'High';
    }
  }
}
