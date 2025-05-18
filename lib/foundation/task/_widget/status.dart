import 'package:flutter/material.dart';
import 'package:flux/foundation/task.dart';

class TaskStatusWidget extends StatefulWidget {
  final TaskStatus status;
  final Function(TaskStatus status) onChange;

  const TaskStatusWidget({
    super.key,
    required this.status,
    required this.onChange,
  });

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
      child: Container(padding: EdgeInsets.all(4.0), child: widget.status.icon),
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
                avatar: option.icon,
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
