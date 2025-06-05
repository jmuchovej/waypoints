import 'package:flutter/material.dart';
import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/things/_task/components/builder.dart';
import 'package:tempo/src/things/_task/node.dart';
import 'package:tempo/src/things/_task/entity.dart';
import 'package:tempo/src/things/_task/components/chips/status.dart';
import 'package:tempo/src/things/_task/components/chips/progress.dart';
import 'package:tempo/src/things/_task/components/chips/due_date.dart';
import 'package:tempo/src/things/_task/components/chips/project.dart';
import 'package:tempo/main.dart' show obx;
import 'package:tempo/src/database/objectbox.g.dart';

/// A document component that displays a complete-able task.
///
/// This is the widget that appears in the document layout for
/// an individual task. This widget includes a checkbox that the
/// user can tap to toggle the completeness of the task.
///
/// The appearance of a [TaskComponent] is configured by the given
/// [viewModel].
class TaskComponent extends StatefulWidget {
  const TaskComponent({
    super.key,
    required this.editor,
    required this.viewModel,
    this.showDebugPaint = false,
  });

  final TaskComponentViewModel viewModel;
  final bool showDebugPaint;
  bool get isSelected => viewModel.isSelected;
  final Editor editor;

  @override
  State<TaskComponent> createState() => TaskComponentState();
}

class TaskComponentState extends State<TaskComponent>
    with ProxyDocumentComponent<TaskComponent>, ProxyTextComposable {
  final _textKey = GlobalKey();

  @override
  GlobalKey<State<StatefulWidget>> get childDocumentComponentKey => _textKey;

  @override
  TextComposable get childTextComposable =>
      childDocumentComponentKey.currentState as TextComposable;

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

  Editor get editor => widget.editor;
  Document get document => editor.document;
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

  Widget _buildPillsRow() {
    final pills = <Widget>[];

    // Check if task has subtasks
    final subtasks = obx.getTasks(
      query: TaskEntity_.parent.equals(node.task.id),
    );

    if (subtasks.isNotEmpty) {
      pills.add(TaskProgressPill(parentTask: node.task));
    }

    // Check if task has due date
    if (pills.isNotEmpty) pills.add(const SizedBox(width: 8));
    pills.add(TaskDueDatePill(task: node.task));

    // Check if task has project
    if (node.task.project.hasValue && node.task.project.target != null) {
      if (pills.isNotEmpty) pills.add(const SizedBox(width: 8));
      pills.add(Flexible(child: TaskProjectPill(task: node.task)));
    }

    // Only show the row if there are pills to display
    if (pills.isEmpty) {
      return const SizedBox.shrink();
    }

    return Padding(
      padding: const EdgeInsets.only(top: 4.0),
      child: Row(
        children: pills,
      ),
    );
  }
}
