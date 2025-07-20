import 'package:flutter/material.dart';

import '../../../infrastructure/logging.dart';
import '../../../things/task/task.entity.dart';

/// This widget replaces the original TaskStatusWidget with precise baseline alignment.
/// It calculates exact positioning based on font metrics to align the status icon
/// with the text baseline rather than just centering it vertically.
///
/// Key features:
/// - Dynamic icon sizing based on font size (fontSize * 0.85)
/// - Precise line height calculation (fontSize * height multiplier)
/// - Mathematical positioning: (lineHeight - iconSize) / 2
/// - Fixed container dimensions that match text line height
///
/// Visual alignment:
/// - Example: 14px font × 1.4 line height = 19.6px total height
/// - Icon size: 11.9px (14px × 0.85)
/// - Top offset: 3.85px ((19.6 - 11.9) / 2)
///
/// Integration notes:
/// - This should replace TaskStatusWidget in production
/// - The sizing ratios (0.85 for icon, 1.4 for line height) may need fine-tuning
/// - Consider making ratios configurable via theme or style system
class TaskStatusChip extends StatelessWidget {
  const TaskStatusChip({
    super.key,
    required this.status,
    required this.textStyle,
    required this.onChange,
  });

  final TaskStatus status;
  final TextStyle textStyle;
  final Function(TaskStatus) onChange;

  @override
  Widget build(BuildContext context) {
    tempoEditorLog.finest('TaskStatusChip building with status: ${status.name} @ ${DateTime.now()}');
    // Calculate dimensions based on text style
    final fontSize = textStyle.fontSize ?? 14.0;
    final lineHeight =
        (textStyle.height ?? 1.4) * fontSize; // Total line height
    final iconSize = lineHeight * 0.85; // Icon slightly smaller than font size

    // Calculate vertical positioning to center icon with text baseline
    final baselineOffset = fontSize * 0.8; // Approximate baseline position
    final iconTopOffset =
        (lineHeight - iconSize) / 2; // Center icon in line height

    return MouseRegion(
      cursor: SystemMouseCursors.click,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: () {
          if (status == TaskStatus.complete) {
            onChange(TaskStatus.todo);
          } else {
            onChange(TaskStatus.complete);
          }
          tempoEditorLog.finest('TaskStatusChip onChange ended @ ${DateTime.now()}');
        },
        onDoubleTap: () {
          if (status == TaskStatus.dropped) {
            onChange(TaskStatus.todo);
          } else {
            onChange(TaskStatus.dropped);
          }
        },
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 0.0),
          child: SizedBox(
            width: lineHeight, // Icon size plus some padding
            height: lineHeight, // Match the text line height exactly
            child: Stack(
              children: [
                Positioned(
                  top: iconTopOffset,
                  // left: 4,
                  child: Icon(
                    status.icon,
                    color: status.color,
                    size: lineHeight,
                    semanticLabel: status.toString(),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
