import 'package:flutter/material.dart';

enum EstimatedTime {
  unknown, // Safety fallback
  XS, // Extra small - almost automatic
  S, // Small - quick task
  M, // Medium - average task
  L, // Large - significant time
  XL, // Extra large - major time commitment
}

void assertStableEnumValues() {
  assert(0 == EstimatedTime.unknown.index);
  assert(1 == EstimatedTime.XS.index);
  assert(2 == EstimatedTime.S.index);
  assert(3 == EstimatedTime.M.index);
  assert(4 == EstimatedTime.L.index);
  assert(5 == EstimatedTime.XL.index);
}

extension EstimatedTimeColors on EstimatedTime {
  Color get color {
    switch (this) {
      case EstimatedTime.XS:
        return Colors.green.shade300;
      case EstimatedTime.S:
        return Colors.green;
      case EstimatedTime.M:
        return Colors.amber;
      case EstimatedTime.L:
        return Colors.orange;
      case EstimatedTime.XL:
        return Colors.red;
      default:
        return Colors.grey;
    }
  }
}

extension EstimatedTimeDisplayString on EstimatedTime {
  String toDisplayString() {
    switch (this) {
      case EstimatedTime.unknown:
        return 'Unknown';
      case EstimatedTime.XS:
        return 'XS';
      case EstimatedTime.S:
        return 'S';
      case EstimatedTime.M:
        return 'M';
      case EstimatedTime.L:
        return 'L';
      case EstimatedTime.XL:
        return 'XL';
    }
  }
}
