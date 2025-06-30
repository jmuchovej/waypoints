import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../editor/editor.dart';
import '../../../editor/styling/text_style_builders.dart';

class PanelHeader extends ConsumerStatefulWidget {
  const PanelHeader({
    super.key,
    required this.titleController,
    required this.titleFocusNode,
    required this.hintText,
    this.children = const [],
  });
  final AttributedTextEditingController titleController;
  final FocusNode titleFocusNode;
  final AttributedText hintText;
  final List<Widget> children;

  @override
  ConsumerState<PanelHeader> createState() => _PanelHeaderState();
}

class _PanelHeaderState extends ConsumerState<PanelHeader> {
  @override
  Widget build(BuildContext context) {
    final textTheme = TextTheme.of(context);
    final bodyLarge = textTheme.bodyLarge!;

    return Padding(
      padding: EdgeInsets.only(
        top: bodyLarge.height!,
        bottom: bodyLarge.height! / 2,
        left: bodyLarge.height! / 2,
        right: bodyLarge.height! / 2,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Center(
            child: ConstrainedBox(
              constraints: BoxConstraints(
                maxWidth: 800,
              ),
              child: TempoTextField(
                focusNode: widget.titleFocusNode,
                controller: widget.titleController,
                hintText: widget.hintText,
                textStyleBuilder: headerTextStyleBuilder(context),
                hintStyleBuilder: headerHintStyleBuilder(context),
              ),
            ),
          ),
          ...widget.children,
        ],
      ),
    );
  }
}
