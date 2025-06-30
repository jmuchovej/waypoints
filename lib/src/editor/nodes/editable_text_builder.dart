import 'package:flutter/material.dart';

import '../editor.dart';
import '../../infrastructure/logging.dart';
import 'editable_text_component.dart';

/// Component builder for EditableTextNode
class EditableTextComponentBuilder implements ComponentBuilder {
  const EditableTextComponentBuilder();

  @override
  Widget? createComponent(
    SingleColumnDocumentComponentContext componentContext,
    SingleColumnLayoutComponentViewModel componentViewModel,
  ) {
    if (componentViewModel is! EditableTextComponentViewModel) {
      return null;
    }

    tempoLog.info(
      '🎨 EditableTextComponentBuilder creating widget for node: ${componentViewModel.nodeId}, '
      'isEditable: ${componentViewModel.isEditable}, textStyleKey: ${componentViewModel.textStyleKey}',
    );

    return EditableTextComponent(
      componentKey: componentContext.componentKey,
      text: componentViewModel.text,
      isEditable: componentViewModel.isEditable,
      textStyleKey: componentViewModel.textStyleKey,
      placeholder: componentViewModel.placeholder,
      icon: componentViewModel.icon,
      showIcon: componentViewModel.showIcon,
      maxLines: componentViewModel.maxLines,
      onTextChanged: componentViewModel.onTextChanged,
      nodeId: componentViewModel.nodeId,
    );
  }

  @override
  SingleColumnLayoutComponentViewModel? createViewModel(
    Document document,
    DocumentNode node,
  ) {
    if (node is! EditableTextNode) {
      return null;
    }

    tempoLog.info(
      '🔧 EditableTextComponentBuilder creating view model for node: ${node.runtimeType}, '
      'id: ${node.id}, blockType: ${node.getMetadataValue("blockType")}',
    );

    return EditableTextComponentViewModel(
      nodeId: node.id,
      text: node.text,
      isEditable: node.isEditable,
      textStyleKey: node.textStyleKey,
      placeholder: node.placeholder,
      icon: node.icon,
      showIcon: node.showIcon,
      maxLines: node.maxLines,
      onTextChanged: node.onTextChanged,
      createdAt: DateTime.now(),
    );
  }
}

/// View model for EditableTextComponent
class EditableTextComponentViewModel
    extends SingleColumnLayoutComponentViewModel {
  EditableTextComponentViewModel({
    required super.nodeId,
    required this.text,
    required this.isEditable,
    required this.textStyleKey,
    required this.placeholder,
    this.icon,
    this.showIcon = true,
    this.maxLines = 1,
    this.onTextChanged,
    super.padding = EdgeInsets.zero,
    super.maxWidth,
    required super.createdAt,
  });

  final AttributedText text;
  final bool isEditable;
  final String textStyleKey;
  final String placeholder;
  final String? icon;
  final bool showIcon;
  final int maxLines;
  final void Function(AttributedText)? onTextChanged;

  @override
  int get hashCode => Object.hash(
    nodeId,
    text,
    isEditable,
    textStyleKey,
    icon,
    showIcon,
    maxLines,
  );

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! EditableTextComponentViewModel) return false;
    if (nodeId != other.nodeId) return false;

    return text == other.text &&
        isEditable == other.isEditable &&
        textStyleKey == other.textStyleKey &&
        icon == other.icon &&
        showIcon == other.showIcon &&
        maxLines == other.maxLines;
  }

  @override
  EditableTextComponentViewModel copy() {
    return EditableTextComponentViewModel(
      nodeId: nodeId,
      text: text,
      isEditable: isEditable,
      textStyleKey: textStyleKey,
      placeholder: placeholder,
      icon: icon,
      showIcon: showIcon,
      maxLines: maxLines,
      onTextChanged: onTextChanged,
      padding: padding,
      maxWidth: maxWidth,
      createdAt: createdAt,
    );
  }
}
