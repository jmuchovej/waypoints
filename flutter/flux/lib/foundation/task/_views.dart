import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flux/foundation/task.dart';
import 'package:flux/ui/editor.dart';

/// Styles all task components to apply top padding
final taskStyles = StyleRule(const BlockSelector("task"), (document, node) {
  if (node is! TaskNode) {
    return {};
  }

  return {Styles.padding: const CascadingPadding.only(top: 24)};
});

class TaskComponentBuilder extends ComponentBuilder {
  TaskComponentBuilder(this._editor);

  final Editor _editor;

  @override
  TaskComponentViewModel? createViewModel(
    Document document,
    DocumentNode node,
  ) {
    if (node is! TaskNode) {
      return null;
    }

    final textDirection = getParagraphDirection(node.name.toPlainText());

    return TaskComponentViewModel(
      nodeId: node.id,
      padding: EdgeInsets.zero,
      indent: node.indent,
      status: node.task.status,
      changeStatus: (TaskStatus status) {
        _editor.execute([
          ChangeTaskStatusRequest(nodeId: node.id, status: status),
        ]);
      },
      text: node.name,
      textDirection: textDirection,
      textAlignment:
          textDirection == TextDirection.ltr ? TextAlign.left : TextAlign.right,
      textStyleBuilder: noStyleBuilder,
      selectionColor: const Color(0x00000000),
    );
  }

  @override
  Widget? createComponent(
    SingleColumnDocumentComponentContext componentContext,
    SingleColumnLayoutComponentViewModel componentViewModel,
  ) {
    if (componentViewModel is! TaskComponentViewModel) {
      return null;
    }

    return TaskComponent(
      key: componentContext.componentKey,
      viewModel: componentViewModel,
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
    required this.viewModel,
    this.showDebugPaint = false,
  });

  final TaskComponentViewModel viewModel;
  final bool showDebugPaint;

  @override
  State<TaskComponent> createState() => _TaskComponentState();
}

class _TaskComponentState extends State<TaskComponent>
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
  Widget build(BuildContext context) {
    return Directionality(
      textDirection: widget.viewModel.textDirection,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // SizedBox(
          //   width: widget.viewModel.indentCalculator(
          //     widget.viewModel.textStyleBuilder({}),
          //     widget.viewModel.indent,
          //   ),
          // ),
          Padding(
            padding: const EdgeInsets.only(left: 8, right: 4),
            child: TaskStatusWidget(
              status: widget.viewModel.status,
              onChange: (status) {
                widget.viewModel.changeStatus(status);
              },
            ),
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
              showDebugPaint: widget.showDebugPaint,
            ),
          ),
        ],
      ),
    );
  }
}
