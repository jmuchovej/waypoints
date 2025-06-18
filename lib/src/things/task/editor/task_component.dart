import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../editor/super_editor.dart';
import '../../../infrastructure/logging.dart';
import '../task_entity.dart';
import '../task_repository.dart';
import 'components/due_date_chip.dart';
import 'components/progress_chip.dart';
import 'components/project_chip.dart';
import 'components/status_chip.dart';
import 'task_builder.dart';
import 'task_node.dart';

/// A document component that displays a complete-able task.
///
/// This is the widget that appears in the document layout for
/// an individual task. This widget includes a checkbox that the
/// user can tap to toggle the completeness of the task.
///
/// The appearance of a [TaskComponent] is configured by the given
/// [viewModel].
class TaskComponent extends ConsumerStatefulWidget {
  const TaskComponent({
    super.key,
    required this.editor,
    required this.viewModel,
    this.showDebugPaint = false,
  });

  final TaskComponentViewModel viewModel;
  final bool showDebugPaint;
  final Editor editor;
  bool get isSelected => viewModel.isSelected;

  @override
  ConsumerState<TaskComponent> createState() => TaskComponentState();
}

class TaskComponentState extends ConsumerState<TaskComponent>
    with ProxyDocumentComponent<TaskComponent>, ProxyTextComposable {
  final _textKey = GlobalKey();

  @override
  GlobalKey<State<StatefulWidget>> get childDocumentComponentKey => _textKey;

  @override
  TextComposable get childTextComposable =>
      childDocumentComponentKey.currentState as TextComposable;

  Document get document => editor.document;

  Editor get editor => widget.editor;

  TaskNode get node =>
      document.getNodeById(widget.viewModel.nodeId) as TaskNode;
  @override
  Widget build(BuildContext context) {
    return Directionality(
      textDirection: widget.viewModel.textDirection,
      child: Padding(
        padding: EdgeInsets.symmetric(vertical: 12.0),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Indentation spacing
            SizedBox(
              width: tempoTaskIndentCalculator(
                widget.viewModel.textStyleBuilder({}),
                widget.viewModel.indent,
              ),
            ),
            // Status widget column
            Column(
              children: [
                TaskStatusChip(
                  status: widget.viewModel.status,
                  textStyle: widget.viewModel.textStyleBuilder({}),
                  onChange: (status) {
                    widget.viewModel.changeStatus(status);
                  },
                ),
              ],
            ),
            const SizedBox(width: 8),
            // Content column (text + progress pill)
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  TextComponent(
                    key: _textKey,
                    text: widget.viewModel.text,
                    textDirection: widget.viewModel.textDirection,
                    textAlign: widget.viewModel.textAlignment,
                    textStyleBuilder: _computeStyles,
                    inlineWidgetBuilders: widget.viewModel.inlineWidgetBuilders,
                    textSelection: widget.viewModel.selection,
                    selectionColor: widget.viewModel.selectionColor,
                    highlightWhenEmpty: widget.viewModel.highlightWhenEmpty,
                    underlines: widget.viewModel.createUnderlines(),
                    showDebugPaint: true,
                  ),
                  // Pills for additional task information
                  _buildPillsRow(),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  @override
  NodePosition? getPositionAtOffset(Offset localOffset) {
    // Get the text component that's nested inside our Row layout
    final textComponentBox =
        childDocumentComponentKey.currentContext?.findRenderObject()
            as RenderBox?;
    if (textComponentBox == null) {
      // Fallback to beginning if we can't find the text component
      return const TextNodePosition(offset: 0);
    }

    // Get our task component's render box
    final taskComponentBox = context.findRenderObject() as RenderBox?;
    if (taskComponentBox == null) {
      return const TextNodePosition(offset: 0);
    }

    try {
      // Convert the click position from task component coordinates to text component coordinates
      // This accounts for the StatusWidget that appears before the text in the Row
      final globalClickPosition = taskComponentBox.localToGlobal(localOffset);
      final textLocalOffset = textComponentBox.globalToLocal(
        globalClickPosition,
      );

      // Now get the position within the text using the adjusted coordinates
      final textComponent =
          childDocumentComponentKey.currentState as TextComponentState?;
      final position = textComponent?.getPositionAtOffset(textLocalOffset);

      // Return the calculated position, or beginning if calculation failed
      return position ?? const TextNodePosition(offset: 0);
    } catch (e) {
      // If coordinate conversion fails, place cursor at beginning
      return const TextNodePosition(offset: 0);
    }
  }

  Widget _buildPillsRow() {
    final pills = <Widget>[];

    // Check if task has subtasks
    final taskRepo = ref.read(taskRepositoryProvider);
    final subtasks = taskRepo.sameTypedChildren(node.task.uuid);

    if (subtasks.isNotEmpty) {
      pills.add(TaskProgressChip(parentTask: node.task));
    }

    // Check if task has due date
    if (pills.isNotEmpty) pills.add(const SizedBox(width: 8));
    pills.add(TaskDueDateChip(task: node.task));

    // Check if task has project
    tempoLog.finest('task project=${node.task.project.target}');
    if (node.task.project.hasValue && node.task.project.target != null) {
      if (pills.isNotEmpty) pills.add(const SizedBox(width: 8));
      pills.add(Flexible(child: TaskProjectChip(task: node.task)));
    }

    // Only show the row if there are pills to display
    if (pills.isEmpty) {
      return const SizedBox.shrink();
    }

    return Padding(
      padding: const EdgeInsets.only(top: 4.0),
      child: Row(children: pills),
    );
  }

  /// Computes the [TextStyle] for this task's inner [TextComponent].
  TextStyle _computeStyles(Set<Attribution> attributions) {
    // Show a strikethrough across the entire task if it's complete.
    final style = widget.viewModel.textStyleBuilder(attributions);
    return widget.viewModel.status.isCompleted
        ? style.copyWith(
          decoration:
              style.decoration == null
                  ? TextDecoration.lineThrough
                  : TextDecoration.combine([
                    TextDecoration.lineThrough,
                    style.decoration!,
                  ]),
        )
        : style;
  }
}
