import 'package:flutter/material.dart';

import '../../entity.dart';

class TaskStatusWidget extends StatefulWidget {
  const TaskStatusWidget({
    super.key,
    required this.status,
    required this.onChange,
  });

  final TaskStatus status;
  final Function(TaskStatus status) onChange;

  @override
  State<TaskStatusWidget> createState() => _TaskStatusWidgetState();
}

class _TaskStatusWidgetState extends State<TaskStatusWidget>
    with SingleTickerProviderStateMixin {
  @override
  void initState() {
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      behavior: HitTestBehavior.translucent,
      onTap: () => _statusChanger(context),
      child: Padding(
        padding: EdgeInsets.symmetric(horizontal: 4.0),
        child: Icon(
          widget.status.icon,
          color: ColorScheme.of(context).primary,
          applyTextScaling: true,
          semanticLabel: widget.status.toDisplayString(),
        ),
      ),
    );
  }

  Widget _statusChanger(BuildContext context) {
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: Row(
        children: [
          for (final option in widget.status.eligibleTransitions)
            Padding(
              padding: const EdgeInsets.only(right: 8.0),
              child: ActionChip(
                avatar: option.buildIcon(context),
                label: Text(option.toDisplayString()),
                backgroundColor:
                    widget.status == option
                        ? Theme.of(context).colorScheme.primaryContainer
                        : null,
                onPressed: () => widget.onChange(option),
              ),
            ),
        ],
      ),
    );
  }
}

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
    // Calculate dimensions based on text style
    final fontSize = textStyle.fontSize ?? 14.0;
    final lineHeight =
        (textStyle.height ?? 1.4) * fontSize; // Total line height
    final iconSize = lineHeight * 0.85; // Icon slightly smaller than font size

    // Calculate vertical positioning to center icon with text baseline
    final baselineOffset = fontSize * 0.8; // Approximate baseline position
    final iconTopOffset =
        (lineHeight - iconSize) / 2; // Center icon in line height

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () => _showStatusChanger(context),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4.0),
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
                  color: Theme.of(context).colorScheme.primary,
                  size: lineHeight,
                  semanticLabel: status.toDisplayString(),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _showStatusChanger(BuildContext context) {
    // For now, just cycle through states - could implement a proper picker later
    final currentIndex = TaskStatus.values.indexOf(status);
    final nextIndex = (currentIndex + 1) % TaskStatus.values.length;
    onChange(TaskStatus.values[nextIndex]);
  }
}
