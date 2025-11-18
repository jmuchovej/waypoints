import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../infrastructure/logging.dart';
import '../../../styling/text_styles.dart';
import '../../../things/task/task.entity.dart';

/// A pill-shaped widget that displays the project association of a task
/// with consistent styling to match the progress pill.
class TaskProjectChip extends ConsumerWidget {
  const TaskProjectChip({super.key, required this.task});

  final TaskEntity task;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Debug logging to understand project relationship state
    waypointsTaskEditorLog.finest('🔍 TaskProjectChip for task ${task.uuid}:');
    waypointsTaskEditorLog.finest('  - hasValue: ${task.project.hasValue}');
    waypointsTaskEditorLog.finest('  - target: ${task.project.target}');
    waypointsTaskEditorLog.finest(
      '  - target title: ${task.project.target?.title.toPlainText()}',
    );

    // If no project, don't show the pill
    if (!task.project.hasValue) {
      waypointsTaskEditorLog.finest(
        '  - No project association (hasValue=false)',
      );
      return const SizedBox.shrink();
    }

    final project = task.project.target;
    if (project == null) {
      waypointsTaskEditorLog.finest('  - Project target is null');
      return const SizedBox.shrink();
    }

    final textStyles = WaypointsTextStyles.of(ref);
    final fontSize = textStyles.chipText.fontSize!;

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
            // TODO convert this to a `WaypointsTextField`
            project.title.toPlainText(),
            style: textStyles.chipText.copyWith(
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
