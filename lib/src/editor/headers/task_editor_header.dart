import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../infrastructure/logging.dart';
import '../editor.dart' hide LogNames;
import '../../ui/widgets/buttons/pill_button.dart';
import '../../ui/widgets/panel/panel_header.dart';
import '../../things/project/project.entity.dart';
import '../../things/task/task.entity.dart';

class TaskEditorHeader extends ConsumerStatefulWidget with PanelHeaderMixin {
  const TaskEditorHeader(
    this.task, {
    required this.focusNode,
    required AttributedTextEditingController controller,
    super.key,
  }) : _controller = controller;

  factory TaskEditorHeader.fromTask(TaskEntity task) {
    final focusNode = FocusNode();
    final controller = AttributedTextEditingController();
    return TaskEditorHeader(task, focusNode: focusNode, controller: controller);
  }

  final TaskEntity task;
  ProjectEntity? get project => task.project.target;
  final FocusNode focusNode;
  final AttributedTextEditingController _controller;

  @override
  AttributedTextEditingController get controller => _controller;

  @override
  ConsumerState<ConsumerStatefulWidget> createState() =>
      _TaskEditorHeaderState();
}

class _TaskEditorHeaderState extends ConsumerState<TaskEditorHeader> {
  @override
  Widget build(BuildContext context) {
    // Debug logging to understand project relationship state
    waypointsTaskEditorLog.finest(
      '🔍 TaskEditorHeader for task ${widget.task.uuid}:',
    );
    waypointsTaskEditorLog.finest(
      '  - hasValue: ${widget.task.project.hasValue}',
    );
    waypointsTaskEditorLog.finest('  - target: ${widget.task.project.target}');
    waypointsTaskEditorLog.finest(
      '  - target title: ${widget.task.project.target?.title.toPlainText()}',
    );
    waypointsTaskEditorLog.finest('  - widget.project: ${widget.project}');

    return PanelHeader(
      titleController: widget._controller,
      titleFocusNode: widget.focusNode,
      text: widget.task.title,
      hintText: AttributedText("Give your task a title"),
      children: [
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            PillButton.icon(
              onPressed: () {
                if (widget.project != null) {
                  // Navigate to the project page (push to maintain navigation stack)
                  context.push('/thing/${widget.project!.uuid}');
                }
              },
              icon: Icon(Icons.folder_outlined),
              label: WaypointsText.fromAttributedText(
                widget.project?.title ?? AttributedText('No Project'),
                sizeMode: WaypointsTextFieldSizeMode.min,
                textStyleBuilder: (attributions) {
                  final theme = Theme.of(context);
                  return TextStyle(
                    color:
                        theme
                            .colorScheme
                            .onPrimary, // FilledButton uses onPrimary for text
                    fontSize: theme.textTheme.labelLarge?.fontSize,
                  );
                },
              ),
            ),
            PillButton.icon(
              onPressed: () => {},
              icon: Icon(Icons.flag_outlined),
              label: WaypointsText.fromAttributedText(
                AttributedText("Priority"),
                sizeMode: WaypointsTextFieldSizeMode.min,
                textStyleBuilder: (attributions) {
                  final theme = Theme.of(context);
                  return TextStyle(
                    color:
                        theme
                            .colorScheme
                            .onPrimary, // FilledButton uses onPrimary for text
                    fontSize: theme.textTheme.labelLarge?.fontSize,
                  );
                },
              ),
            ),
            PillButton.icon(
              onPressed: () => {},
              icon: Icon(Icons.timer_outlined),
              label: WaypointsText.fromAttributedText(
                AttributedText("Time"),
                sizeMode: WaypointsTextFieldSizeMode.min,
                textStyleBuilder: (attributions) {
                  final theme = Theme.of(context);
                  return TextStyle(
                    color:
                        theme
                            .colorScheme
                            .onPrimary, // FilledButton uses onPrimary for text
                    fontSize: theme.textTheme.labelLarge?.fontSize,
                  );
                },
              ),
            ),
            PillButton.icon(
              onPressed: () => {},
              icon: Icon(Icons.battery_charging_full_outlined),
              label: WaypointsText.fromAttributedText(
                AttributedText("Energy"),
                sizeMode: WaypointsTextFieldSizeMode.min,
                textStyleBuilder: (attributions) {
                  final theme = Theme.of(context);
                  return TextStyle(
                    color:
                        theme
                            .colorScheme
                            .onPrimary, // FilledButton uses onPrimary for text
                    fontSize: theme.textTheme.labelLarge?.fontSize,
                  );
                },
              ),
            ),
          ],
        ),
      ],
    );
  }
}
