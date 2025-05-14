import 'package:flutter/material.dart';
import 'package:iconify/iconify.dart';
import 'package:iconify/icons/phosphor.dart';

enum Priority {
  unknown, // Safety fallback
  L, // Low priority
  M, // Medium priority
  H, // High priority
}

void assertStableEnumValues() {
  assert(0 == Priority.unknown.index);
  assert(1 == Priority.L.index);
  assert(2 == Priority.M.index);
  assert(3 == Priority.H.index);
}

extension PriorityColors on Priority {
  Color get color {
    switch (this) {
      case Priority.L:
        return Colors.green;
      case Priority.M:
        return Colors.orange;
      case Priority.H:
        return Colors.red;
      default:
        return Colors.grey;
    }
  }
}

extension PriorityIcons on Priority {
  Widget get icon {
    String label = Phosphor["exclamation-mark-fill"];
    return Iconify(label, color: color);
  }
}

extension PriortyDisplayString on Priority {
  String toDisplayString() {
    switch (this) {
      case Priority.unknown:
        return 'Unknown';
      case Priority.L:
        return 'Low';
      case Priority.M:
        return 'Medium';
      case Priority.H:
        return 'High';
    }
  }
}
