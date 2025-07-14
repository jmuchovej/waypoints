import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../editor/editor.dart';
import '../../../editor/styling/text_style_builders.dart';
import '../../../styling/tempo_stylesheet.dart';

class PanelHeader extends ConsumerStatefulWidget {
  const PanelHeader({
    super.key,
    required this.titleController,
    required this.titleFocusNode,
    required this.hintText,
    this.children = const [],
    this.readonly = false,
  });

  /// Constructor for read-only titles
  PanelHeader.readonly({
    super.key,
    required AttributedText title,
    this.children = const [],
  }) : titleController = AttributedTextEditingController(text: title),
       titleFocusNode = FocusNode(),
       hintText = title,
       readonly = true;

  final AttributedTextEditingController titleController;
  final FocusNode titleFocusNode;
  final AttributedText hintText;
  final List<Widget> children;
  final bool readonly;

  @override
  ConsumerState<PanelHeader> createState() => _PanelHeaderState();
}

class _PanelHeaderState extends ConsumerState<PanelHeader> {
  @override
  Widget build(BuildContext context) {
    final stylesheet = TempoStylesheet.of(ref);
    final textStyles = stylesheet.textStyles;
    final spacing = stylesheet.spacing;

    final styleBuilder = headerTextStyleBuilder(context);
    final headerText =
        widget.readonly
            ? TempoText.fromAttributedText(widget.hintText, textStyleBuilder: styleBuilder, sizeMode: TempoTextFieldSizeMode.max,)
            : TempoTextField.fromAttributedText(
              widget.hintText,
              textStyleBuilder: styleBuilder,
              controller: widget.titleController,
              focusNode: widget.titleFocusNode,
              hintStyleBuilder: headerHintStyleBuilder(context),
            );

    return ConstrainedBox(
      constraints: BoxConstraints(maxWidth: spacing.maxContentWidth),
      child: Padding(
        padding: EdgeInsets.only(
          top:
              textStyles
                  .contentBody
                  .fontSize!, // Top padding for spacing from app bar
          bottom:
              textStyles.contentBody.fontSize! /
              2, // Bottom padding before SuperEditor content
          left: 0, // No horizontal padding - handled by parent container
          right: 0,
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Padding(
              padding: EdgeInsetsGeometry.only(
                bottom: widget.children.isNotEmpty ?  textStyles.documentHeader.fontSize! / 2 : 0,
              ),
              child: headerText,
            ),
            ...widget.children,
          ],
        ),
      ),
    );
  }
}
