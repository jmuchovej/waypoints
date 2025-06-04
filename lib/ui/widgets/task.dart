import 'package:flutter/material.dart';
import 'package:jiffy/jiffy.dart';
import 'package:tempo/main.dart';
import 'package:tempo/src/things/task.dart';
import 'package:tempo/src/editor.dart';

class TaskWidget extends StatefulWidget {
  const TaskWidget({
    super.key,
    required this.task,
    required this.onTap,
    this.showDueDate = false,
  });

  final TaskEntity task;
  final bool showDueDate;
  final void Function() onTap;

  @override
  TaskWidgetState createState() => TaskWidgetState();
}

class TaskWidgetState extends State<TaskWidget>
    with SingleTickerProviderStateMixin {
  bool _openedPanelStatus = false;
  bool _openedPanelPriority = false;
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

  void _togglePanelStatus() {
    if (_openedPanelPriority) {
      _openedPanelPriority = false;
    }

    _openedPanelStatus = !_openedPanelStatus;
    _togglePanel();
  }

  void _togglePanelPriority() {
    if (_openedPanelStatus) {
      _openedPanelStatus = false;
    }

    _openedPanelPriority = !_openedPanelPriority;
    _togglePanel();
  }

  void _togglePanel() {
    setState(() {
      if (_openedPanelStatus || _openedPanelPriority) {
        _controller.forward();
      } else {
        _controller.reverse();
      }
    });
  }

  void _closePanels() {
    setState(() {
      _openedPanelStatus = false;
      _openedPanelPriority = false;
      _controller.reverse();
    });
  }

  void _updateTask() {
    widget.task.modifyDate = DateTime.now();
    obx.putTask(widget.task);

    // widget.onTap();
    _closePanels();
  }

  void _updateStatus(TaskStatus status) {
    widget.task.obxStatus = TaskStatus.values.indexOf(status);
    _updateTask();
  }

  void _updatePriority(TaskPriority priority) {
    widget.task.obxPriority = TaskPriority.values.indexOf(priority);
    _updateTask();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [_buildTaskTile(context), _buildEditPanel(context)],
    );
  }

  Widget _buildTaskTile(BuildContext context) {
    final controller = AttributedTextEditingController(text: widget.task.title);
    final dueDate = Jiffy.parseFromDateTime(widget.task.dueDate);
    final subtitle = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // if (widget.task.description.isNotEmpty)
        //   Text(
        //     widget.task.description,
        //     maxLines: 1,
        //     overflow: TextOverflow.ellipsis,
        //   ),
        if (widget.showDueDate)
          Text('Due ${dueDate.fromNow()}', style: TextStyle(fontSize: 12)),
      ],
    );
    return ListTile(
      title: TempoTextField(controller: controller),
      subtitle: subtitle,
      isThreeLine: subtitle.children.length == 2,
      leading: GestureDetector(
        behavior: HitTestBehavior.translucent,
        onTap: _togglePanelStatus,
        child: Container(
          padding: const EdgeInsets.all(4.0),
          child: widget.task.status.buildIcon(context),
        ),
      ),
      trailing: GestureDetector(
        behavior: HitTestBehavior.translucent,
        onTap: _togglePanelPriority,
        child: Container(
          padding: const EdgeInsets.all(4.0),
          child: widget.task.priority.icon,
        ),
      ),
      onTap: widget.onTap,
    );
  }

  Widget _buildEditPanel(BuildContext context) {
    return SizeTransition(
      sizeFactor: _animation,
      child: Container(
        width: MediaQuery.of(context).size.width,
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
            if (_openedPanelStatus)
              ..._buildPanelSelect<TaskStatus>(
                context,
                "Status:",
                widget.task.status,
                widget.task.status.eligibleTransitions,
                _updateStatus,
                (status) => status.buildIcon(context),
                (status) => status.toDisplayString(),
              ),

            if (_openedPanelPriority)
              ..._buildPanelSelect<TaskPriority>(
                context,
                "Priority:",
                widget.task.priority,
                TaskPriority.values.toList(),
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
