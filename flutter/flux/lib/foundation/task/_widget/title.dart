import 'package:flutter/material.dart';
import 'package:flux/foundation/task.dart';
import 'package:flux/editor.dart';

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
      textSelection: widget.viewModel.selection,
      selectionColor: widget.viewModel.selectionColor,
      highlightWhenEmpty: widget.viewModel.highlightWhenEmpty,
      underlines: widget.viewModel.createUnderlines(),
    );
  }
}
