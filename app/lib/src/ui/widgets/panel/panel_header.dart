import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../editor/editor.dart';
import '../../../editor/styling/text_style_builders.dart';
import '../../../styling/stylesheet.dart';

mixin PanelHeaderMixin on ConsumerStatefulWidget {
  AttributedTextEditingController get controller;
}

class PanelHeader extends ConsumerStatefulWidget with PanelHeaderMixin {
  PanelHeader({
    super.key,
    required this.titleController,
    required this.titleFocusNode,
    AttributedText? text,
    this.hintText,
    this.children = const [],
    this.readonly = false,
  }) {
    if (text != null) {
      titleController.update(text: text);
    }
    if (!readonly) {
      assert(hintText != null);
    }
  }

  /// Constructor for read-only titles
  factory PanelHeader.readonly({
    required AttributedText title,
    List<Widget> children = const [],
  }) {
    final titleController = AttributedTextEditingController(text: title);
    final titleFocusNode = FocusNode();
    return PanelHeader(
      titleController: titleController,
      titleFocusNode: titleFocusNode,
      readonly: true,
      children: children,
    );
  }

  final AttributedTextEditingController titleController;
  final FocusNode titleFocusNode;
  final AttributedText? hintText;
  final List<Widget> children;
  final bool readonly;

  @override
  AttributedTextEditingController get controller => titleController;

  @override
  ConsumerState<PanelHeader> createState() => _PanelHeaderState();
}

class _PanelHeaderState extends ConsumerState<PanelHeader> {
  @override
  Widget build(BuildContext context) {
    final stylesheet = WaypointsStylesheet.of(ref);
    final textStyles = stylesheet.textStyles;
    final spacing = stylesheet.spacing;

    final styleBuilder = headerTextStyleBuilder(context);
    final headerText = widget.readonly
        ? WaypointsText.fromAttributedText(
            widget.controller.text,
            textStyleBuilder: styleBuilder,
            sizeMode: WaypointsTextFieldSizeMode.max,
          )
        : WaypointsTextField.fromAttributedText(
            widget.hintText!,
            textStyleBuilder: styleBuilder,
            controller: widget.titleController,
            focusNode: widget.titleFocusNode,
            hintStyleBuilder: headerHintStyleBuilder(context),
          );

    return ConstrainedBox(
      constraints: BoxConstraints(maxWidth: spacing.maxContentWidth),
      child: Padding(
        padding: EdgeInsets.only(
          top: textStyles
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
                bottom: widget.children.isNotEmpty
                    ? textStyles.documentHeader.fontSize! / 2
                    : 0,
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
