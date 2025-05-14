import 'package:flutter/material.dart';
import 'package:flux/objectbox.dart';
import 'package:flux/ui/screens/task_editor.dart' show TaskEditorScreen;
import 'package:flux/utils/extensions.dart';
import 'package:flux/utils/ui_helpers.dart';

import '../../models/task.dart';

class TaskWidget extends StatefulWidget {
  final Task task;
  final void Function() onTap;

  const TaskWidget({super.key, required this.task, required this.onTap});

  @override
  TaskWidgetState createState() => TaskWidgetState();
}

class TaskWidgetState extends State<TaskWidget>
    with SingleTickerProviderStateMixin {
  bool _isExpanded = false;
  late AnimationController _controller;
  late Animation<double> _animation;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      duration: const Duration(milliseconds: 200),
      vsync: this,
    );

    _animation = CurvedAnimation(parent: _controller, curve: Curves.easeInOut);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _togglePanel() {
    setState(() {
      _isExpanded = !_isExpanded;
      if (_isExpanded) {
        _controller.forward();
      } else {
        _controller.reverse();
      }
    });
  }

  void _updateStatus(Status status) {
    final box = ObjectBox.taskBox();
    widget.task.status = status;
    widget.task.modifyDate = DateTime.now();
    box.put(widget.task);

    widget.onTap();

    _togglePanel();
  }

  void _updatePriority(Priority priority) {
    final box = ObjectBox.taskBox();
    widget.task.priority = priority;
    widget.task.modifyDate = DateTime.now();
    box.put(widget.task);

    widget.onTap();

    _togglePanel();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [_buildTaskTile(), _buildEditPanel()],
    );
  }

  Widget _buildTaskTile() {
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
        onTap: _togglePanel,
        child: Container(
          padding: const EdgeInsets.all(4.0),
          child: UIHelpers.buildStatusIcon(widget.task.status),
        ),
      ),
      trailing: GestureDetector(
        behavior: HitTestBehavior.translucent,
        onTap: _togglePanel,
        child: Container(
          padding: const EdgeInsets.all(4.0),
          child: UIHelpers.buildPriorityIndicator(widget.task.priority),
        ),
      ),
      onTap: widget.onTap,
    );
  }

  Widget _buildEditPanel() {
    return SizeTransition(
      sizeFactor: _animation,
      child: Container(
        padding: EdgeInsets.symmetric(horizontal: 16.0, vertical: 8.0),
        decoration: BoxDecoration(
          color: Colors.grey.shade100,
          border: Border(
            top: BorderSide(color: Colors.grey.shade300),
            bottom: BorderSide(color: Colors.grey.shade300),
          ),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            ..._buildPanelSelect<Status>(
              context,
              "Status:",
              widget.task.status,
              Status.values.where((s) => s != Status.unknown).toList(),
              _updateStatus,
              (status) => status.icon,
              (status) => status.toDisplayString(),
            ),
            SizedBox(height: 8.0),
            ..._buildPanelSelect<Priority>(
              context,
              "Priority:",
              widget.task.priority,
              Priority.values.where((p) => p != Priority.unknown).toList(),
              _updatePriority,
              (priority) => priority.icon,
              (priority) => priority.toDisplayString(),
            ),
          ],
        ),
      ),
    );
  }

  List<Widget> _buildPanelSelect<T>(
    BuildContext context,
    String label,
    T? current,
    List<T> options,
    Function(T) update,
    Widget Function(T) iconBuilder,
    String Function(T) stringBuilder,
  ) {
    return [
      Text(label, style: TextStyle(fontWeight: FontWeight.bold)),
      SizedBox(height: 4.0),
      SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: [
            for (final option in options)
              Padding(
                padding: const EdgeInsets.only(right: 8.0),
                child: ActionChip(
                  avatar: iconBuilder(option),
                  label: Text(stringBuilder(option)),
                  backgroundColor:
                      current == option
                          ? Theme.of(context).colorScheme.primaryContainer
                          : null,
                  onPressed: () => update(option),
                ),
              ),
          ],
        ),
      ),
    ];
  }
}
