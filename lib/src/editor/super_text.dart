import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:super_editor_markdown/super_editor_markdown.dart';
import 'package:super_text_layout/super_text_layout.dart';

import 'super_editor.dart';

export 'package:super_text_layout/super_text_layout.dart';

/// Sizing behavior for TempoTextField
enum TempoTextFieldSizeMode {
  /// Expands to fill available width (default behavior)
  adaptive,

  /// Sizes to minimum required width for content
  min,

  /// Sizes to maximum available width
  max,
}

/// Base class for all Tempo text widgets providing common interface
abstract class TempoTextBase extends StatelessWidget {
  const TempoTextBase({
    super.key,
    required this.text,
    this.textStyleBuilder,
    this.textAlign = TextAlign.left,
    this.textDirection = TextDirection.ltr,
    this.sizeMode = TempoTextFieldSizeMode.adaptive,
  });

  final AttributedText text;
  final AttributionStyleBuilder? textStyleBuilder;
  final TextAlign textAlign;
  final TextDirection textDirection;
  final TempoTextFieldSizeMode sizeMode;
}

/// A display-only widget for AttributedText using SuperText
/// This is much more efficient than TempoTextField for read-only text
///
/// TODO: Eventually mask Flutter's built-in Text widget to enforce
/// consistent AttributedText usage throughout the app. This will ensure
/// all text rendering goes through SuperText for unified rich text support.
class TempoText extends TempoTextBase {
  const TempoText({
    super.key,
    required super.text,
    super.textStyleBuilder,
    super.textAlign = TextAlign.left,
    super.textDirection = TextDirection.ltr,
    super.sizeMode = TempoTextFieldSizeMode.adaptive,
  });

  const TempoText.fromAttributedText(
    AttributedText text, {
    super.key,
    AttributionStyleBuilder? textStyleBuilder,
    TextAlign textAlign = TextAlign.left,
    TextDirection textDirection = TextDirection.ltr,
    TempoTextFieldSizeMode sizeMode = TempoTextFieldSizeMode.adaptive,
  }) : super(
         text: text,
         textStyleBuilder: textStyleBuilder,
         textAlign: textAlign,
         textDirection: textDirection,
         sizeMode: sizeMode,
       );

  AttributionStyleBuilder get _defaultStyleBuilder {
    return (Set<Attribution> attributions) {
      TextStyle style = const TextStyle(fontSize: 16, color: Colors.black);

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
              backgroundColor: Colors.grey.shade200,
            );
            break;
          case ColorAttribution(:final color):
            style = style.copyWith(color: color);
            break;
          case BackgroundColorAttribution(:final color):
            style = style.copyWith(backgroundColor: color);
            break;
          case FontSizeAttribution(:final fontSize):
            style = style.copyWith(fontSize: fontSize);
            break;
          case FontFamilyAttribution(:final fontFamily):
            style = style.copyWith(fontFamily: fontFamily);
            break;
        }
      }

      return style;
    };
  }

  @override
  Widget build(BuildContext context) {
    final styleBuilder = textStyleBuilder ?? _defaultStyleBuilder;
    final richText = text.computeInlineSpan(
      context,
      styleBuilder,
      defaultInlineWidgetBuilderChain,
    );

    final superText = SuperText(
      richText: richText,
      textAlign: textAlign,
      textDirection: textDirection,
    );

    // Apply sizing wrapper based on sizeMode
    switch (sizeMode) {
      case TempoTextFieldSizeMode.min:
        return IntrinsicWidth(child: superText);
      case TempoTextFieldSizeMode.max:
        return SizedBox(width: double.infinity, child: superText);
      case TempoTextFieldSizeMode.adaptive:
        return superText;
    }
  }
}

/// TODO: Eventually mask Flutter's built-in TextField widget to enforce
/// consistent AttributedText usage throughout the app. This will ensure
/// all text input goes through SuperTextField for unified rich text editing.
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
    this.sizeMode = TempoTextFieldSizeMode.adaptive,
    this.textAlign = TextAlign.left,
    this.textDirection = TextDirection.ltr,
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
    AttributedTextEditingController? controller,
    this.sizeMode = TempoTextFieldSizeMode.adaptive,
    this.textAlign = TextAlign.left,
    this.textDirection = TextDirection.ltr,
  }) : controller = controller ?? AttributedTextEditingController(text: text);

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
  final TempoTextFieldSizeMode sizeMode;
  final TextAlign textAlign;
  final TextDirection textDirection;

  // Computed properties that match TempoTextBase interface
  AttributedText get text => controller.text;

  /// Access to the controller - useful for external management
  AttributedTextEditingController get textController => controller;

  /// Access to the focus node - useful for external management
  FocusNode? get textFocusNode => focusNode;

  @override
  State<TempoTextField> createState() => _TempoTextFieldState();
}

class _TempoTextFieldState extends State<TempoTextField> {
  late FocusNode _focusNode;
  String? _previousPlainText;

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

  @override
  Widget build(BuildContext context) {
    final textField = SuperTextField(
      textController: widget.controller,
      imeConfiguration: TextInputConfiguration(readOnly: widget.readOnly),
      focusNode: _focusNode,
      textAlign: widget.textAlign,
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

    // Apply sizing wrapper based on sizeMode
    switch (widget.sizeMode) {
      case TempoTextFieldSizeMode.min:
        return IntrinsicWidth(child: textField);
      case TempoTextFieldSizeMode.max:
        return SizedBox(width: double.infinity, child: textField);
      case TempoTextFieldSizeMode.adaptive:
        return textField;
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
}
