import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:tempo/src/editor/super_editor.dart';

import 'package:tempo/src/things/_task/entity.dart';
import 'package:tempo/src/things/_task/node.dart';
import 'package:tempo/src/things/_task/components/component.dart';

class TaskComponentBuilder extends ComponentBuilder {
  TaskComponentBuilder([this.editor]);
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
      padding: EdgeInsets.all(4),
      status: node.task.status,
      changeStatus: (TaskStatus status) {},
      text: node.title,
      textDirection: textDirection,
      textAlignment:
          textDirection == TextDirection.ltr ? TextAlign.left : TextAlign.right,
      textStyleBuilder: noStyleBuilder,
      indent: node.indent,
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
    required this.status,
    required this.changeStatus,
    required this.text,
    required this.textStyleBuilder,
    this.indent = 0,
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

  TaskStatus status;
  void Function(TaskStatus) changeStatus;

  /// The indent level of this task - `0` is no indent.
  int indent;

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
      status: status,
      changeStatus: changeStatus,
      text: text,
      textStyleBuilder: textStyleBuilder,
      indent: indent,
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

/// Calculates the indentation width for Tempo tasks.
///
/// This uses a spacing of approximately 24 pixels per indent level,
/// which provides good visual hierarchy for nested tasks.
double tempoTaskIndentCalculator(TextStyle textStyle, int indent) {
  return 24.0 * indent;
}
