import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:super_editor_markdown/super_editor_markdown.dart';
import 'package:tempo/src/editor/super_editor.dart';

export 'package:super_text_layout/super_text_layout.dart';

class TempoTextField extends StatefulWidget {
  const TempoTextField({
    super.key,
    required this.controller,
    this.hintText,
    this.readOnly = false,
    this.textStyleBuilder,
    this.hintStyleBuilder,
    this.onMarkdownConversion,
    this.minLines,
    this.maxLines,
    this.padding,
    this.focusNode,
  });

  TempoTextField.fromAttributedText(
    AttributedText text, {
    super.key,
    this.hintText,
    this.readOnly = false,
    this.textStyleBuilder,
    this.hintStyleBuilder,
    this.onMarkdownConversion,
    this.minLines,
    this.maxLines,
    this.padding,
    this.focusNode,
  }) : controller = AttributedTextEditingController(text: text);

  final AttributedTextEditingController controller;
  final AttributedText? hintText;
  final bool readOnly;
  final AttributionStyleBuilder? textStyleBuilder;
  final AttributionStyleBuilder? hintStyleBuilder;
  final void Function(AttributedText)? onMarkdownConversion;
  final int? minLines;
  final int? maxLines;
  final EdgeInsets? padding;
  final FocusNode? focusNode;

  @override
  State<TempoTextField> createState() => _TempoTextFieldState();
}

class _TempoTextFieldState extends State<TempoTextField> {
  late FocusNode _focusNode;
  String? _previousPlainText;

  @override
  void initState() {
    super.initState();
    _focusNode = widget.focusNode ?? FocusNode();
    _focusNode.canRequestFocus = !widget.readOnly;
    _previousPlainText = widget.controller.text.toPlainText();

    // Listen for text changes to apply markdown
    if (!widget.readOnly) {
      widget.controller.addListener(_onTextChanged);
    }
  }

  @override
  void dispose() {
    if (!widget.readOnly) {
      widget.controller.removeListener(_onTextChanged);
    }
    // Only dispose the focus node if we created it (not passed from parent)
    if (widget.focusNode == null) {
      _focusNode.dispose();
    }
    super.dispose();
  }

  void _onTextChanged() {
    final currentText = widget.controller.text.toPlainText();
    if (currentText == _previousPlainText) {
      return; // No actual text change, just attribution changes
    }

    // Convert markdown syntax to attributions
    final markdownText = attributedTextFromMarkdown(currentText);

    if (markdownText.toPlainText() != currentText) {
      // Markdown conversion changed the text (e.g., removed asterisks)
      final selection = widget.controller.selection;
      widget.controller.text = markdownText;

      // Try to maintain cursor position
      if (selection.isCollapsed) {
        final newPosition = selection.extent.offset;
        widget.controller.selection = TextSelection.collapsed(
          offset: newPosition.clamp(0, markdownText.length),
        );
      }

      widget.onMarkdownConversion?.call(markdownText);
    } else if (markdownText.isEmpty) {
      // Only attributions changed
      widget.controller.text = markdownText;
    }

    _previousPlainText = widget.controller.text.toPlainText();
  }

  @override
  Widget build(BuildContext context) {
    return SuperTextField(
      textController: widget.controller,
      imeConfiguration: TextInputConfiguration(readOnly: widget.readOnly),
      focusNode: _focusNode,
      // showCursor: !widget.readOnly,

      // Hint text support
      hintBehavior: HintBehavior.displayHintUntilTextEntered,
      hintBuilder:
          widget.hintText != null
              ? (context) => Text(
                widget.hintText!.toPlainText(),
                style: (widget.hintStyleBuilder ?? _defaultHintStyleBuilder)(
                  widget.hintText!.getAllAttributionsAt(0),
                ),
              )
              : null,

      // Text styling
      textStyleBuilder: widget.textStyleBuilder ?? _defaultTextStyleBuilder,

      // Remove selection for read-only
      // selectionControls: widget.readOnly ? null : materialTextSelectionControls,
      minLines: widget.minLines,
      maxLines: widget.maxLines,
      padding: widget.padding,
    );
  }

  AttributionStyleBuilder get _defaultTextStyleBuilder {
    return (Set<Attribution> attributions) {
      final theme = Theme.of(context);
      final colorScheme = theme.colorScheme;

      TextStyle style = TextStyle(
        fontSize: theme.textTheme.bodyLarge?.fontSize ?? 16,
        color: colorScheme.onSurface,
        fontFamily: theme.textTheme.bodyLarge?.fontFamily,
      );

      // Apply attributions
      for (final attribution in attributions) {
        switch (attribution) {
          case boldAttribution:
            style = style.copyWith(fontWeight: FontWeight.bold);
            break;
          case italicsAttribution:
            style = style.copyWith(fontStyle: FontStyle.italic);
            break;
          case underlineAttribution:
            style = style.copyWith(decoration: TextDecoration.underline);
            break;
          case strikethroughAttribution:
            style = style.copyWith(decoration: TextDecoration.lineThrough);
            break;
          case const NamedAttribution('code'):
            style = style.copyWith(
              fontFamily: 'monospace',
              backgroundColor: colorScheme.surfaceContainerHighest,
            );
            break;
          case ColorAttribution(:final color):
            style = style.copyWith(color: color);
            break;
          case BackgroundColorAttribution(:final color):
            style = style.copyWith(backgroundColor: color);
            break;
          case LinkAttribution():
            style = style.copyWith(
              color: colorScheme.primary,
              decoration: TextDecoration.underline,
            );
            break;
        }
      }

      return style;
    };
  }

  AttributionStyleBuilder get _defaultHintStyleBuilder {
    return (Set<Attribution> attributions) {
      final theme = Theme.of(context);
      return TextStyle(
        fontSize: theme.textTheme.bodyLarge?.fontSize ?? 16,
        color: theme.colorScheme.onSurface.withValues(alpha: 0.5),
        fontFamily: theme.textTheme.bodyLarge?.fontFamily,
      );
    };
  }
}
