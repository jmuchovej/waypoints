import 'package:flutter/material.dart';
import 'package:tempo/flavors.dart';

class DebugBorder extends StatelessWidget {
  const DebugBorder({super.key, required this.child, this.color = Colors.red});

  final Widget child;
  final Color color;

  @override
  Widget build(BuildContext context) {
    if (F.appFlavor != Flavor.development) {
      return child;
    }

    return DecoratedBox(
      decoration: BoxDecoration(border: Border.all(color: color, width: 2)),
      child: child,
    );
  }
}
