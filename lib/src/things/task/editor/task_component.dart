import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../editor/operations/thing_cursor_handler.dart';
import '../../../editor/super_editor.dart';
import '../../../infrastructure/logging.dart';
import '../../thing/thing_node_position.dart';
import '../task_entity.dart';
import '../task_repository.dart';
import 'components/due_date_chip.dart';
import 'components/progress_chip.dart';
import 'components/project_chip.dart';
import 'components/status_chip.dart';
import 'task_builder.dart';

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
    required this.viewModel,
    this.showDebugPaint = false,
  });

  final TaskComponentViewModel viewModel;
  final bool showDebugPaint;
  bool get isSelected => viewModel.isSelected;

  @override
  ConsumerState<TaskComponent> createState() => TaskComponentState();
}

class TaskComponentState extends ConsumerState<TaskComponent>
    with
        ProxyDocumentComponent<TaskComponent>,
        ProxyTextComposable,
        ThingCursorProvider {
  final _textKey = GlobalKey();

  // Debug flag - set to true to see detailed gesture logging
  static const bool _debugGestures = true;

  TaskEntity get task => widget.viewModel.task;

  @override
  GlobalKey<State<StatefulWidget>> get childDocumentComponentKey => _textKey;

  @override
  TextComposable get childTextComposable =>
      childDocumentComponentKey.currentState as TextComposable;

  // Note: Pointer handling is now managed by ThingTitleComponent
  // with context-aware interaction patterns from the settings system

  @override
  Widget build(BuildContext context) {
    final textSize = TextTheme.of(context).bodyMedium!.fontSize!;

    return Directionality(
      textDirection: widget.viewModel.textDirection,
      child: Padding(
        padding: EdgeInsets.symmetric(vertical: textSize / 2),
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
                  // Use context-aware ThingTitleComponent for all platforms
                  TextComponent(
                    key: _textKey,
                    // thing: task,
                    text: widget.viewModel.text,
                    textDirection: widget.viewModel.textDirection,
                    textAlign: widget.viewModel.textAlignment,
                    textStyleBuilder: _computeStyles,
                    inlineWidgetBuilders: widget.viewModel.inlineWidgetBuilders,
                    textSelection: widget.viewModel.selection,
                    selectionColor: widget.viewModel.selectionColor,
                    highlightWhenEmpty: widget.viewModel.highlightWhenEmpty,
                    underlines: widget.viewModel.createUnderlines(),
                    showDebugPaint: widget.showDebugPaint,
                    // onEdit: () {
                    //   if (_debugGestures) {
                    //     tempoLog.info('✏️ ThingTitleComponent edit for task: ${task.uuid}');
                    //   }
                    //   // Focus the component for editing
                    //   // The SuperEditor system will handle the actual editing
                    // },
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
    // Determine which cursor zone this offset is in
    final cursorZone = getCursorZoneForOffset(localOffset);

    if (cursorZone == ThingCursorZone.textEditable) {
      // For text areas, return the actual text position
      return _getTextComponentPositionAtOffset(localOffset);
    } else {
      // For non-text areas, return a ThingNodePosition with zone metadata
      // Use -1 offset as convention for non-text positions
      return ThingNodePosition(offset: -1, zone: cursorZone);
    }
  }

  bool _isClickInTextArea(Offset localOffset) {
    try {
      // Get the text component that's nested inside our Row layout
      final textComponentBox =
          childDocumentComponentKey.currentContext?.findRenderObject()
              as RenderBox?;
      if (textComponentBox == null) {
        if (_debugGestures) {
          tempoLog.finest('🎯 _isClickInTextArea: No text component box found');
        }
        return false;
      }

      // Get our task component's render box
      final taskComponentBox = context.findRenderObject() as RenderBox?;
      if (taskComponentBox == null) {
        if (_debugGestures) {
          tempoLog.finest('🎯 _isClickInTextArea: No task component box found');
        }
        return false;
      }

      // Convert the local offset to the text component's coordinate space
      final textOffset = taskComponentBox.globalToLocal(
        textComponentBox.localToGlobal(Offset.zero),
      );

      // Check if the click is within the text component bounds
      final textBounds = Rect.fromLTWH(
        textOffset.dx,
        textOffset.dy,
        textComponentBox.size.width,
        textComponentBox.size.height,
      );

      final isInTextArea = textBounds.contains(localOffset);

      if (_debugGestures) {
        tempoLog.finest(
          '🎯 _isClickInTextArea: click=$localOffset, textBounds=$textBounds, result=$isInTextArea',
        );
      }

      return isInTextArea;
    } catch (e) {
      if (_debugGestures) {
        tempoLog.warning(
          '🎯 _isClickInTextArea: Error calculating text area: $e',
        );
      }
      // Default to false (not in text area) on any error
      return false;
    }
  }

  NodePosition? _getTextComponentPositionAtOffset(Offset localOffset) {
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
    final subtasks = taskRepo.sameTypedChildren(task.uuid);

    if (subtasks.isNotEmpty) {
      pills.add(TaskProgressChip(parentTask: task));
    }

    // Check if task has due date
    if (pills.isNotEmpty) pills.add(const SizedBox(width: 8));
    pills.add(TaskDueDateChip(task: task));

    // Check if task has project
    // tempoLog.finest('task project=${node.task.project.target}');
    if (task.project.hasValue && task.project.target != null) {
      if (pills.isNotEmpty) pills.add(const SizedBox(width: 8));
      pills.add(Flexible(child: TaskProjectChip(task: task)));
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

  @override
  ThingCursorZone getCursorZoneForOffset(Offset localOffset) {
    try {
      // Status chip area (left side) - interactive
      if (localOffset.dx < 40) {
        if (_debugGestures) {
          tempoLog.finest('🎯 Cursor zone: status chip area (interactive)');
        }
        return ThingCursorZone.interactive;
      }

      // Check if we're in the text area
      if (_isClickInTextArea(localOffset)) {
        if (_debugGestures) {
          tempoLog.finest('🎯 Cursor zone: text area (editable)');
        }
        return ThingCursorZone.textEditable;
      }

      // Pills/chips area (right side) - interactive
      final componentBox = context.findRenderObject() as RenderBox?;
      if (componentBox != null) {
        final rightZoneStart = componentBox.size.width * 0.7; // Approximate
        if (localOffset.dx > rightZoneStart) {
          if (_debugGestures) {
            tempoLog.finest('🎯 Cursor zone: pills/chips area (interactive)');
          }
          return ThingCursorZone.interactive;
        }
      }

      // Everything else is clickable (for navigation)
      if (_debugGestures) {
        tempoLog.finest('🎯 Cursor zone: general area (clickable)');
      }
      return ThingCursorZone.clickable;
    } catch (e) {
      if (_debugGestures) {
        tempoLog.warning('🎯 Error determining cursor zone: $e');
      }
      // Default to clickable on any error
      return ThingCursorZone.clickable;
    }
  }
}
