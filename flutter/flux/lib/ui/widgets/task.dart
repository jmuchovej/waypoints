import 'package:flutter/material.dart';
import 'package:flux/ui/screens/task_editor.dart' show TaskEditorScreen;
import 'package:flux/utils/ui_helpers.dart';

import '../../models/task.dart';

class TaskWidget extends StatefulWidget {
  final Task task;
  final void Function() onTap;

  const TaskWidget({super.key, required this.task, required this.onTap});

  @override
  TaskWidgetState createState() => TaskWidgetState();
}

class TaskWidgetState extends State<TaskWidget> {
  @override
  Widget build(BuildContext context) {
    return ListTile(
      title: Text(
        widget.task.name,
        style: TextStyle(
          decoration:
              widget.task.status.isCompleted
                  ? TextDecoration.lineThrough
                  : null,
        ),
      ),
      subtitle:
          widget.task.description.isEmpty
              ? null
              : Text(
                widget.task.description,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
      leading: GestureDetector(
        behavior: HitTestBehavior.translucent,
        onTap: () {},
        child: Container(
          padding: const EdgeInsets.all(4.0),
          child: UIHelpers.buildStatusIcon(widget.task.status),
        ),
      ),
      trailing: GestureDetector(
        behavior: HitTestBehavior.translucent,
        onTap: () {},
        child: Container(
          padding: const EdgeInsets.all(4.0),
          child: UIHelpers.buildPriorityIndicator(widget.task.priority),
        ),
      ),
      onTap: widget.onTap,
    );
  }
}
