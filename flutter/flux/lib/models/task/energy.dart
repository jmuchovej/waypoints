import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';

enum EstimatedEnergy {
  unknown, // Safety fallback
  XS, // Almost no energy
  S, // Little energy
  M, // Medium energy
  L, // Significant energy
  XL, // Peak energy required
}

void assertStableEnumValues() {
  assert(0 == EstimatedEnergy.unknown.index);
  assert(1 == EstimatedEnergy.XS.index);
  assert(2 == EstimatedEnergy.S.index);
  assert(3 == EstimatedEnergy.M.index);
  assert(4 == EstimatedEnergy.L.index);
  assert(5 == EstimatedEnergy.XL.index);
}

extension EstimatedEnergyColors on EstimatedEnergy {
  Color get color {
    switch (this) {
      case EstimatedEnergy.XS:
        return Colors.green.shade300;
      case EstimatedEnergy.S:
        return Colors.green;
      case EstimatedEnergy.M:
        return Colors.amber;
      case EstimatedEnergy.L:
        return Colors.orange;
      case EstimatedEnergy.XL:
        return Colors.red;
      default:
        return Colors.grey;
    }
  }
}

extension EstimatedEnergyDisplayString on EstimatedEnergy {
  String toDisplayString() {
    switch (this) {
      case EstimatedEnergy.unknown:
        return 'Unknown';
      case EstimatedEnergy.XS:
        return 'XS';
      case EstimatedEnergy.S:
        return 'S';
      case EstimatedEnergy.M:
        return 'M';
      case EstimatedEnergy.L:
        return 'L';
      case EstimatedEnergy.XL:
        return 'XL';
    }
  }
}
