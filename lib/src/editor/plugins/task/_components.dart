import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:tempo/src/database/objectbox/models/task.dart';
import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/infrastructure/_logging.dart';

/// Styles all task components to apply top padding
final taskStyles = StyleRule(taskSelector, (document, node) {
  if (node is! TaskNode) {
    return {};
  }

  return {Styles.padding: const CascadingPadding.only(top: 12)};
});

class TaskComponentBuilder extends ComponentBuilder {
  TaskComponentBuilder();
  Editor? editor;

  @override
  TaskComponentViewModel? createViewModel(
    Document document,
    DocumentNode node,
  ) {
    if (node is! TaskNode) {
      return null;
    }

    final textDirection = getParagraphDirection(node.title.toPlainText());

    return TaskComponentViewModel(
      nodeId: node.id,
      padding: EdgeInsets.zero,
      indent: node.indent,
      status: node.task.status,
      changeStatus: (TaskStatus status) {},
      text: node.title,
      textDirection: textDirection,
      textAlignment:
          textDirection == TextDirection.ltr ? TextAlign.left : TextAlign.right,
      textStyleBuilder: noStyleBuilder,
      selectionColor: Colors.blue.shade300,
    );
  }

  @override
  Widget? createComponent(
    SingleColumnDocumentComponentContext componentContext,
    SingleColumnLayoutComponentViewModel componentViewModel,
  ) {
    if (componentViewModel is! TaskComponentViewModel || editor == null) {
      return null;
    }

    return TaskComponent(
      key: componentContext.componentKey,
      viewModel: componentViewModel,
      editor: editor as Editor,
      showDebugPaint: true,
    );
  }
}

class TaskComponentViewModel extends SingleColumnLayoutComponentViewModel
    with TextComponentViewModel {
  TaskComponentViewModel({
    required super.nodeId,
    super.maxWidth,
    required super.padding,
    this.indent = 0,
    this.indentCalculator = defaultTaskIndentCalculator,
    required this.status,
    required this.changeStatus,
    required this.text,
    required this.textStyleBuilder,
    this.inlineWidgetBuilders = const [],
    this.textDirection = TextDirection.ltr,
    this.textAlignment = TextAlign.left,
    this.isSelected = false,
    this.selection,
    required this.selectionColor,
    this.highlightWhenEmpty = false,
    TextRange? composingRegion,
    bool showComposingRegionUnderline = false,
    UnderlineStyle spellingErrorUnderlineStyle = const SquiggleUnderlineStyle(
      color: Colors.red,
    ),
    List<TextRange> spellingErrors = const <TextRange>[],
    UnderlineStyle grammarErrorUnderlineStyle = const SquiggleUnderlineStyle(
      color: Colors.blue,
    ),
    List<TextRange> grammarErrors = const <TextRange>[],
  }) {
    this.composingRegion = composingRegion;
    this.showComposingRegionUnderline = showComposingRegionUnderline;

    this.spellingErrorUnderlineStyle = spellingErrorUnderlineStyle;
    this.spellingErrors = spellingErrors;

    this.grammarErrorUnderlineStyle = grammarErrorUnderlineStyle;
    this.grammarErrors = grammarErrors;
  }

  int indent;
  TextBlockIndentCalculator indentCalculator;

  TaskStatus status;
  void Function(TaskStatus) changeStatus;

  @override
  AttributedText text;
  @override
  AttributionStyleBuilder textStyleBuilder;
  @override
  InlineWidgetBuilderChain inlineWidgetBuilders;
  @override
  TextDirection textDirection;
  @override
  TextAlign textAlignment;
  bool isSelected;
  @override
  TextSelection? selection;
  @override
  Color selectionColor;
  @override
  bool highlightWhenEmpty;

  @override
  TaskComponentViewModel copy() {
    return TaskComponentViewModel(
      nodeId: nodeId,
      maxWidth: maxWidth,
      padding: padding,
      indent: indent,
      indentCalculator: indentCalculator,
      status: status,
      changeStatus: changeStatus,
      text: text,
      textStyleBuilder: textStyleBuilder,
      inlineWidgetBuilders: inlineWidgetBuilders,
      textDirection: textDirection,
      textAlignment: textAlignment,
      selection: selection,
      selectionColor: selectionColor,
      highlightWhenEmpty: highlightWhenEmpty,
      spellingErrorUnderlineStyle: spellingErrorUnderlineStyle,
      spellingErrors: List.from(spellingErrors),
      grammarErrorUnderlineStyle: grammarErrorUnderlineStyle,
      grammarErrors: List.from(grammarErrors),
      composingRegion: composingRegion,
      showComposingRegionUnderline: showComposingRegionUnderline,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      super == other &&
          other is TaskComponentViewModel &&
          runtimeType == other.runtimeType &&
          indent == other.indent &&
          status == other.status &&
          text == other.text &&
          textDirection == other.textDirection &&
          textAlignment == other.textAlignment &&
          selection == other.selection &&
          selectionColor == other.selectionColor &&
          highlightWhenEmpty == other.highlightWhenEmpty &&
          spellingErrorUnderlineStyle == other.spellingErrorUnderlineStyle &&
          const DeepCollectionEquality().equals(
            spellingErrors,
            other.spellingErrors,
          ) &&
          grammarErrorUnderlineStyle == other.grammarErrorUnderlineStyle &&
          const DeepCollectionEquality().equals(
            grammarErrors,
            other.grammarErrors,
          ) &&
          composingRegion == other.composingRegion &&
          showComposingRegionUnderline == other.showComposingRegionUnderline;

  @override
  int get hashCode =>
      super.hashCode ^
      indent.hashCode ^
      status.hashCode ^
      text.hashCode ^
      textDirection.hashCode ^
      textAlignment.hashCode ^
      selection.hashCode ^
      selectionColor.hashCode ^
      highlightWhenEmpty.hashCode ^
      spellingErrorUnderlineStyle.hashCode ^
      spellingErrors.hashCode ^
      grammarErrorUnderlineStyle.hashCode ^
      grammarErrors.hashCode ^
      composingRegion.hashCode ^
      showComposingRegionUnderline.hashCode;
}

/// The standard [TextBlockIndentCalculator] used by tasks in `SuperEditor`.
double defaultTaskIndentCalculator(TextStyle textStyle, int indent) {
  return (textStyle.fontSize! * 0.60) * 4 * indent;
}

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
    final node = document.getNodeById(widget.viewModel.nodeId) as TaskNode;

    if (!node.inEditMode) {
      return TextNodePosition(offset: 0);
    }

    return super.getPositionAtOffset(localOffset);
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
      child: Column(
        mainAxisAlignment: MainAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 4,
            children: [
              // SizedBox(
              //   width: widget.viewModel.indentCalculator(
              //     widget.viewModel.textStyleBuilder({}),
              //     widget.viewModel.indent,
              //   ),
              // ),
              TaskStatusWidget(
                status: widget.viewModel.status,
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
                  // textSelection: widget.viewModel.selection,
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
      child: Padding(
        padding: EdgeInsets.symmetric(horizontal: 4.0),
        child: Icon(
          widget.status.icon,
          color: ColorScheme.of(context).primary,
          applyTextScaling: true,
          semanticLabel: widget.status.toDisplayString(),
        ),
      ),
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
                avatar: option.buildIcon(context),
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

class TaskTitleWidget extends StatefulWidget {
  final TaskComponentViewModel viewModel;
  final GlobalKey<State<StatefulWidget>> textKey;

  const TaskTitleWidget({
    super.key,
    required this.viewModel,
    required this.textKey,
  });

  @override
  State<TaskTitleWidget> createState() => _TaskTitleWidgetState();
}

class _TaskTitleWidgetState extends State<TaskTitleWidget>
    with SingleTickerProviderStateMixin {
  @override
  void initState() {
    super.initState();
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
  Widget build(BuildContext context) {
    return TextComponent(
      key: widget.textKey,
      text: widget.viewModel.text,
      textDirection: widget.viewModel.textDirection,
      textAlign: widget.viewModel.textAlignment,
      textStyleBuilder: _computeStyles,
      inlineWidgetBuilders: widget.viewModel.inlineWidgetBuilders,
      // textSelection: widget.viewModel.selection,
      selectionColor: widget.viewModel.selectionColor,
      highlightWhenEmpty: widget.viewModel.highlightWhenEmpty,
      underlines: widget.viewModel.createUnderlines(),
    );
  }
}
