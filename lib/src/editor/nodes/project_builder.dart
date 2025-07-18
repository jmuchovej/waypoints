import 'package:flutter/material.dart';
import 'package:super_editor/super_editor.dart';

import '../../infrastructure/logging.dart';
import '../../things/project/project.entity.dart';
import 'project_component.dart';
import 'project_node.dart';

/// Component builder for ProjectNode
class ProjectComponentBuilder implements ComponentBuilder {
  const ProjectComponentBuilder();

  @override
  SingleColumnLayoutComponentViewModel? createViewModel(
    Document document,
    DocumentNode node,
  ) {
    if (node is! ProjectNode) {
      return null;
    }

    final textDirection = getParagraphDirection(node.title.toPlainText());

    return ProjectComponentViewModel(
      nodeId: node.id,
      padding: EdgeInsets.all(4),
      project: node.project,
      text: node.project.title,
      textDirection: textDirection,
      textAlignment:
          textDirection == TextDirection.ltr ? TextAlign.left : TextAlign.right,
      selectionColor: Colors.blue.shade300,
      createdAt: node.project.createDate,
    );
  }

  @override
  Widget? createComponent(
    SingleColumnDocumentComponentContext componentContext,
    SingleColumnLayoutComponentViewModel componentViewModel,
  ) {
    if (componentViewModel is! ProjectComponentViewModel) {
      return null;
    }

    return ProjectComponent(
      key: componentContext.componentKey,
      viewModel: componentViewModel,
      // selection: componentViewModel.selection,
    );
  }
}

/// View model for the project component
class ProjectComponentViewModel extends SingleColumnLayoutComponentViewModel
    with TextComponentViewModel {
  ProjectComponentViewModel({
    required super.nodeId,
    required this.project,
    required this.text,
    super.maxWidth,
    super.padding = EdgeInsets.zero,
    this.textDirection = TextDirection.ltr,
    this.textAlignment = TextAlign.left,
    this.selection,
    this.indent = 0,
    required this.selectionColor,
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
    required super.createdAt,
  }) {
    this.composingRegion = composingRegion;
    this.showComposingRegionUnderline = showComposingRegionUnderline;

    this.spellingErrorUnderlineStyle = spellingErrorUnderlineStyle;
    this.spellingErrors = spellingErrors;

    this.grammarErrorUnderlineStyle = grammarErrorUnderlineStyle;
    this.grammarErrors = grammarErrors;
  }

  final ProjectEntity project;
  final int indent;

  @override
  bool highlightWhenEmpty = false;

  @override
  InlineWidgetBuilderChain inlineWidgetBuilders = [];

  @override
  TextSelection? selection;

  @override
  Color selectionColor;

  @override
  AttributedText text;

  @override
  TextAlign textAlignment;

  @override
  TextDirection textDirection;

  @override
  AttributionStyleBuilder textStyleBuilder = noStyleBuilder;

  @override
  ProjectComponentViewModel copy() {
    return ProjectComponentViewModel(
      nodeId: nodeId,
      project: project,
      maxWidth: maxWidth,
      padding: padding,
      text: text,
      textDirection: textDirection,
      textAlignment: textAlignment,
      selection: selection,
      selectionColor: selectionColor,
      composingRegion: composingRegion,
      showComposingRegionUnderline: showComposingRegionUnderline,
      spellingErrorUnderlineStyle: spellingErrorUnderlineStyle,
      spellingErrors: List.from(spellingErrors),
      grammarErrorUnderlineStyle: grammarErrorUnderlineStyle,
      grammarErrors: List.from(grammarErrors),
      createdAt: createdAt,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      super == other &&
          other is ProjectComponentViewModel &&
          runtimeType == other.runtimeType &&
          project.uuid == other.project.uuid;

  @override
  int get hashCode => super.hashCode ^ project.uuid.hashCode;
}
