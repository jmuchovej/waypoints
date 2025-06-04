import 'package:flutter/material.dart';
import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/infrastructure.dart';

import 'package:tempo/src/things/_task/components/builder.dart';
import 'package:tempo/src/things/_task/node.dart';
import 'package:tempo/src/things/_task/entity.dart';
import 'package:tempo/src/things/_task/components/picker_status.dart';

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

  void _onTap() {
    if (!node.inEditMode) {
      editor.execute([
        ChangeSelectionRequest(
          DocumentSelection(
            base: DocumentPosition(
              nodeId: node.id,
              nodePosition: const TextNodePosition(offset: 0),
            ),
            extent: DocumentPosition(
              nodeId: node.id,
              nodePosition: TextNodePosition(offset: node.text.length),
            ),
          ),
          SelectionChangeType.placeCaret,
          SelectionReason.userInteraction,
        ),
      ]);
    }
  }

  @override
  Widget build(BuildContext context) {
    // return Container(
    //   decoration: BoxDecoration(
    //     borderRadius: BorderRadius.circular(8),
    //     color: widget.isSelected ? widget.viewModel.selectionColor : null,
    //   ),
    // pluginTaskLog.finest('size: ${context.size}');
    return Directionality(
      textDirection: widget.viewModel.textDirection,
      child: Padding(
        padding: EdgeInsets.symmetric(vertical: 12.0),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.start,
          children: [
            // CLAUDE CODE MODIFICATION: Changed to baseline alignment for proper icon positioning
            Row(
              crossAxisAlignment:
                  CrossAxisAlignment
                      .baseline, // Changed from .start to .baseline
              textBaseline:
                  TextBaseline.alphabetic, // Required for baseline alignment
              spacing: 4,
              children: [
                // SizedBox(
                //   width: widget.viewModel.indentCalculator(
                //     widget.viewModel.textStyleBuilder({}),
                //     widget.viewModel.indent,
                //   ),
                // ),
                BaselineAlignedTaskStatusWidget(
                  status: widget.viewModel.status,
                  textStyle: widget.viewModel.textStyleBuilder(
                    {},
                  ), // Pass text style for sizing
                  onChange: (status) {
                    widget.viewModel.changeStatus(status);
                  },
                ),
                Expanded(
                  child: TextComponent(
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
                ),
              ],
            ),
            // if (widget.isSelected) Row(children: [_buildQuickActions()]),
          ],
        ),
      ),
    );
  }

  Widget _buildQuickActions() {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        IconButton(
          icon: Icon(Icons.calendar_today, size: 18),
          onPressed: () {
            // Show date picker for due date
          },
        ),
        IconButton(
          icon: Icon(Icons.flag, size: 18),
          onPressed: () {
            // Show priority selector
          },
        ),
        // Add other quick actions
      ],
    );
  }

  void _handleTaskTap() {
    final nodeID = widget.viewModel.nodeId;
    pluginTaskLog.fine("task tapped! nodeID=$nodeID");
    // if (!isSelectedectionManager.isTaskSelected(nodeID)) {
    //   widget.selectionManager.selectTask(nodeID);
    //   logger.d("selected task! nodeID=$nodeID");
    //   return;
    // }

    // logger.d("task nodeID=$nodeID is: ${widget.viewModel.isSelected}");
    // TODO implement editing mode on the task itself...
  }
}
