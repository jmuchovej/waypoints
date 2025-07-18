import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:jiffy/jiffy.dart';

import '../../../styling/text_styles.dart';
import '../../../things/task/task.entity.dart';

/// A pill-shaped widget that displays the due date of a task
/// with consistent styling to match the progress pill.
class TaskDueDateChip extends ConsumerWidget {
  const TaskDueDateChip({super.key, required this.task});

  final TaskEntity task;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dueDate = task.dueDate;
    final now = DateTime.now();
    final isOverdue = dueDate.isBefore(now);
    final isToday =
        dueDate.year == now.year &&
        dueDate.month == now.month &&
        dueDate.day == now.day;

    final textStyles = TempoTextStyles.of(ref);
    final fontSize = textStyles.chipText.fontSize!;

    // Determine colors based on due date status
    Color iconColor;
    if (isOverdue) {
      iconColor = Theme.of(context).colorScheme.error;
    } else if (isToday) {
      iconColor = Theme.of(context).colorScheme.tertiary;
    } else {
      iconColor = Theme.of(context).colorScheme.onSurfaceVariant;
    }

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(
          Icons.calendar_today_rounded,
          size: 1.25 * fontSize,
          color: iconColor,
        ),
        SizedBox(width: 0.5 * fontSize),
        Text(
          Jiffy.parseFromDateTime(dueDate).fromNow(),
          style: textStyles.chipText.copyWith(
            color: iconColor,
            fontSize: fontSize,
            fontWeight: FontWeight.w500,
          ),
        ),
      ],
    );
  }
}
