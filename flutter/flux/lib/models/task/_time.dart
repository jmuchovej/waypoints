import 'package:flutter/material.dart';

enum TaskTimeEstimate {
  XS, // Extra small - almost automatic
  S, // Small - quick task
  M, // Medium - average task
  L, // Large - significant time
  XL, // Extra large - major time commitment
}

void assertStableEnumValues() {
  assert(0 == TaskTimeEstimate.XS.index);
  assert(1 == TaskTimeEstimate.S.index);
  assert(2 == TaskTimeEstimate.M.index);
  assert(3 == TaskTimeEstimate.L.index);
  assert(4 == TaskTimeEstimate.XL.index);
}

extension EstimatedTimeColors on TaskTimeEstimate {
  Color get color {
    switch (this) {
      case TaskTimeEstimate.XS:
        return Colors.green.shade300;
      case TaskTimeEstimate.S:
        return Colors.green;
      case TaskTimeEstimate.M:
        return Colors.amber;
      case TaskTimeEstimate.L:
        return Colors.orange;
      case TaskTimeEstimate.XL:
        return Colors.red;
    }
  }
}

extension EstimatedTimeDisplayString on TaskTimeEstimate {
  String toDisplayString() {
    switch (this) {
      case TaskTimeEstimate.XS:
        return 'XS';
      case TaskTimeEstimate.S:
        return 'S';
      case TaskTimeEstimate.M:
        return 'M';
      case TaskTimeEstimate.L:
        return 'L';
      case TaskTimeEstimate.XL:
        return 'XL';
    }
  }
}
