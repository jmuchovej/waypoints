import 'package:flutter/material.dart';

enum TaskEnergyEstimate {
  XS, // Almost no energy
  S, // Little energy
  M, // Medium energy
  L, // Significant energy
  XL; // Peak energy required

  static void assertStableEnumValues() {
    assert(0 == TaskEnergyEstimate.XS.index);
    assert(1 == TaskEnergyEstimate.S.index);
    assert(2 == TaskEnergyEstimate.M.index);
    assert(3 == TaskEnergyEstimate.L.index);
    assert(4 == TaskEnergyEstimate.XL.index);
  }
}

extension EstimatedEnergyColors on TaskEnergyEstimate {
  Color get color {
    switch (this) {
      case TaskEnergyEstimate.XS:
        return Colors.green.shade300;
      case TaskEnergyEstimate.S:
        return Colors.green;
      case TaskEnergyEstimate.M:
        return Colors.amber;
      case TaskEnergyEstimate.L:
        return Colors.orange;
      case TaskEnergyEstimate.XL:
        return Colors.red;
    }
  }
}

extension EstimatedEnergyDisplayString on TaskEnergyEstimate {
  String toDisplayString() {
    switch (this) {
      case TaskEnergyEstimate.XS:
        return 'XS';
      case TaskEnergyEstimate.S:
        return 'S';
      case TaskEnergyEstimate.M:
        return 'M';
      case TaskEnergyEstimate.L:
        return 'L';
      case TaskEnergyEstimate.XL:
        return 'XL';
    }
  }
}
