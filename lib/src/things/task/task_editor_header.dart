import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../editor/editor.dart';
import '../../editor/super_editor.dart';
import '../../styling/tempo_stylesheet.dart';
import '../../ui/widgets/buttons/pill_button.dart';
import '../../ui/widgets/panel/panel_header.dart';
import '../project/project_entity.dart';
import 'task_entity.dart';

class TaskEditorHeader extends ConsumerStatefulWidget {
  const TaskEditorHeader(
    this.task, {
    required this.focusNode,
    required this.controller,
    super.key,
  });

  factory TaskEditorHeader.fromTask(TaskEntity task) {
    final focusNode = FocusNode();
    final controller = AttributedTextEditingController();
    return TaskEditorHeader(task, focusNode: focusNode, controller: controller);
  }

  final TaskEntity task;
  ProjectEntity? get project => task.project.target;
  final FocusNode focusNode;
  final AttributedTextEditingController controller;

  @override
  ConsumerState<ConsumerStatefulWidget> createState() =>
      _TaskEditorHeaderState();
}

class _TaskEditorHeaderState extends ConsumerState<TaskEditorHeader> {
  @override
  Widget build(BuildContext context) {
    final hintText = AttributedText("New Task");
    return PanelHeader(
      titleController: widget.controller,
      titleFocusNode: widget.focusNode,
      hintText: hintText,
      children: [
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            PillButton.icon(
              onPressed: () => {},
              icon: Icon(Icons.folder_outlined),
              label: TempoText.fromAttributedText(
                widget.project?.title ?? AttributedText('No Project'),
                sizeMode: TempoTextFieldSizeMode.min,
                textStyleBuilder: (attributions) {
                  final theme = Theme.of(context);
                  return TextStyle(
                    color: theme.colorScheme.onPrimary, // FilledButton uses onPrimary for text
                    fontSize: theme.textTheme.labelLarge?.fontSize,
                  );
                },
              ),
            ),
            PillButton.icon(
              onPressed: () => {},
              icon: Icon(Icons.flag_outlined),
              label: TempoText.fromAttributedText(
                AttributedText("Priority"),
                sizeMode: TempoTextFieldSizeMode.min,
                textStyleBuilder: (attributions) {
                  final theme = Theme.of(context);
                  return TextStyle(
                    color: theme.colorScheme.onPrimary, // FilledButton uses onPrimary for text
                    fontSize: theme.textTheme.labelLarge?.fontSize,
                  );
                },
              ),
            ),
            PillButton.icon(
              onPressed: () => {},
              icon: Icon(Icons.timer_outlined),
              label: TempoText.fromAttributedText(
                AttributedText("Time"),
                sizeMode: TempoTextFieldSizeMode.min,
                textStyleBuilder: (attributions) {
                  final theme = Theme.of(context);
                  return TextStyle(
                    color: theme.colorScheme.onPrimary, // FilledButton uses onPrimary for text
                    fontSize: theme.textTheme.labelLarge?.fontSize,
                  );
                },
              ),
            ),
            PillButton.icon(
              onPressed: () => {},
              icon: Icon(Icons.battery_charging_full_outlined),
              label: TempoText.fromAttributedText(
                AttributedText("Energy"),
                sizeMode: TempoTextFieldSizeMode.min,
                textStyleBuilder: (attributions) {
                  final theme = Theme.of(context);
                  return TextStyle(
                    color: theme.colorScheme.onPrimary, // FilledButton uses onPrimary for text
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
