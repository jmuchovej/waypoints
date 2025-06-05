import 'package:flutter/material.dart';
import 'package:tempo/src/things/_task/entity.dart';
import 'package:tempo/main.dart' show obx;

/// A pill-shaped widget that displays the project association of a task
/// with consistent styling to match the progress pill.
class TaskProjectPill extends StatelessWidget {
  const TaskProjectPill({
    super.key,
    required this.task,
  });

  final TaskEntity task;

  @override
  Widget build(BuildContext context) {
    // If no project, don't show the pill
    if (!task.project.hasValue) {
      return const SizedBox.shrink();
    }

    final project = task.project.target;
    if (project == null) {
      return const SizedBox.shrink();
    }

    final fontSize = Theme.of(context).textTheme.labelMedium!.fontSize!;

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(
          Icons.folder_rounded,
          size: 1.25 * fontSize,
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
        SizedBox(width: 0.5 * fontSize),
        Flexible(
          child: Text(
            project.title.text,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
              fontSize: fontSize,
              fontWeight: FontWeight.w500,
            ),
            overflow: TextOverflow.ellipsis,
            maxLines: 1,
          ),
        ),
      ],
    );
  }
}