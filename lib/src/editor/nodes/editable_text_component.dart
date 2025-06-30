import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../editor.dart';
import '../../infrastructure/logging.dart';
import '../styling/text_style_builders.dart';

/// Visual component for EditableTextNode
class EditableTextComponent extends StatefulWidget {
  const EditableTextComponent({
    super.key,
    required this.componentKey,
    required this.text,
    required this.isEditable,
    required this.textStyleKey,
    required this.placeholder,
    this.icon,
    this.showIcon = true,
    this.maxLines = 1,
    this.onTextChanged,
    required this.nodeId,
  });

  final GlobalKey componentKey;
  final AttributedText text;
  final bool isEditable;
  final String textStyleKey;
  final String placeholder;
  final String? icon;
  final bool showIcon;
  final int maxLines;
  final void Function(AttributedText)? onTextChanged;
  final String nodeId;

  @override
  State<EditableTextComponent> createState() => _EditableTextComponentState();
}

class _EditableTextComponentState extends State<EditableTextComponent> {
  late TextEditingController _textController;
  final FocusNode _focusNode = FocusNode();
  bool _isEditing = false;

  @override
  void initState() {
    super.initState();
    _textController = TextEditingController(text: widget.text.toPlainText());
    _focusNode.addListener(_onFocusChanged);
  }

  @override
  void didUpdateWidget(EditableTextComponent oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.text != widget.text && !_isEditing) {
      _textController.text = widget.text.toPlainText();
    }
  }

  @override
  void dispose() {
    _focusNode.removeListener(_onFocusChanged);
    _focusNode.dispose();
    _textController.dispose();
    super.dispose();
  }

  void _onFocusChanged() {
    if (!_focusNode.hasFocus && _isEditing) {
      _finishEditing();
    }
  }

  void _startEditing() {
    if (!widget.isEditable) return;

    setState(() {
      _isEditing = true;
    });

    // Select all text when starting to edit
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _focusNode.requestFocus();
      _textController.selection = TextSelection(
        baseOffset: 0,
        extentOffset: _textController.text.length,
      );
    });
  }

  void _finishEditing() {
    if (!_isEditing) return;

    setState(() {
      _isEditing = false;
    });

    // Update the node with new text
    final newText = AttributedText(_textController.text);
    if (newText.toPlainText() != widget.text.toPlainText()) {
      // Just call the callback - the parent widget will handle the update
      widget.onTextChanged?.call(newText);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    tempoLog.fine(
      '🖼️ EditableTextComponent building widget with text: "${widget.text.toPlainText()}", '
      'isEditable: ${widget.isEditable}, isEditing: $_isEditing',
    );

    return BoxComponent(
      key: widget.componentKey,
      isVisuallySelectable: false,
      child: GestureDetector(
        onTap: widget.isEditable ? _startEditing : null,
        behavior: HitTestBehavior.opaque,
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            // Icon (if provided and showIcon is true)
            if (widget.icon != null && widget.showIcon) ...[
              Icon(
                _getIconData(widget.icon!),
                size: _getIconSize(),
                color: theme.colorScheme.primary,
              ),
              const SizedBox(width: 16),
            ],

            // Text content
            Expanded(
              child: _isEditing ? _buildEditingField() : _buildStaticText(),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildStaticText() {
    final isEmpty = widget.text.toPlainText().isEmpty;
    
    if (isEmpty && widget.isEditable) {
      // Show placeholder
      return Text(
        widget.placeholder,
        style: _getTextStyle(context).copyWith(
          color: Theme.of(context).colorScheme.onSurface.withValues(alpha: 0.5),
        ),
        maxLines: widget.maxLines,
        overflow: TextOverflow.ellipsis,
      );
    }

    return TextComponent(
      text: widget.text,
      textStyleBuilder: _getStyleBuilder(context),
    );
  }

  Widget _buildEditingField() {
    return TextField(
      controller: _textController,
      focusNode: _focusNode,
      style: _getTextStyle(context),
      maxLines: widget.maxLines,
      decoration: InputDecoration(
        hintText: widget.placeholder,
        border: InputBorder.none,
        contentPadding: EdgeInsets.zero,
        isDense: true,
      ),
      onSubmitted: (_) => _finishEditing(),
      onEditingComplete: _finishEditing,
      inputFormatters: [
        if (widget.maxLines == 1)
          FilteringTextInputFormatter.deny(RegExp(r'\n')),
      ],
    );
  }

  TextStyle _getTextStyle(BuildContext context) {
    return _getStyleBuilder(context)({});
  }

  TextStyleBuilder _getStyleBuilder(BuildContext context) {
    switch (widget.textStyleKey) {
      case 'documentHeader':
      case 'pageTitle':
        return headerTextStyleBuilder(context);
      case 'header1':
        return header1TextStyleBuilder(context);
      case 'header2':
        return header2TextStyleBuilder(context);
      case 'header3':
        return header3TextStyleBuilder(context);
      default:
        return defaultTextStyleBuilder(context);
    }
  }

  double _getIconSize() {
    switch (widget.textStyleKey) {
      case 'documentHeader':
      case 'pageTitle':
        return 32;
      case 'header1':
        return 28;
      case 'header2':
        return 24;
      default:
        return 20;
    }
  }

  IconData _getIconData(String iconName) {
    switch (iconName) {
      case 'tasks':
        return Icons.format_list_bulleted_rounded;
      case 'projects':
        return Icons.folder_rounded;
      case 'notes':
        return Icons.notes_rounded;
      case 'calendar':
        return Icons.calendar_today_rounded;
      case 'inbox':
        return Icons.inbox_rounded;
      case 'today':
        return Icons.today_rounded;
      default:
        return Icons.article_rounded;
    }
  }
}