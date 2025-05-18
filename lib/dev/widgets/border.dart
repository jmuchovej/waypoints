import 'package:flutter/material.dart';
import 'package:flux/config/environment.dart';

class DebugBorder extends StatelessWidget {
  final Widget child;
  final Color color;

  const DebugBorder({super.key, required this.child, this.color = Colors.red});

  @override
  Widget build(BuildContext context) {
    if (!EnvironmentConfig.isDevelopment) {
      return child;
    }

    return Container(
      decoration: BoxDecoration(border: Border.all(color: color, width: 2)),
      child: child,
    );
  }
}
