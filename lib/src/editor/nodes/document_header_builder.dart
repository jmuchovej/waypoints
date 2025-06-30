import 'package:flutter/material.dart';

import '../editor.dart';
import '../../infrastructure/logging.dart';
import 'document_header_component.dart';

/// Component builder for DocumentHeaderNode
class DocumentHeaderComponentBuilder implements ComponentBuilder {
  const DocumentHeaderComponentBuilder();

  @override
  Widget? createComponent(
    SingleColumnDocumentComponentContext componentContext,
    SingleColumnLayoutComponentViewModel componentViewModel,
  ) {
    if (componentViewModel is! DocumentHeaderComponentViewModel) {
      return null;
    }

    // Log when DocumentHeader component widget is created with padding info
    tempoLog.info(
      '🎨 DocumentHeaderComponentBuilder creating widget for node: ${componentViewModel.nodeId}, '
      'padding: ${componentViewModel.padding}',
    );

    return DocumentHeaderComponent(
      componentKey: componentContext.componentKey,
      title: componentViewModel.title,
      icon: componentViewModel.icon,
    );
  }

  @override
  SingleColumnLayoutComponentViewModel? createViewModel(
    Document document,
    DocumentNode node,
  ) {
    if (node is! DocumentHeaderNode) {
      return null;
    }

    // Log when DocumentHeader component builder creates view model
    tempoLog.info(
      '🔧 DocumentHeaderComponentBuilder creating view model for node: ${node.runtimeType}, '
      'id: ${node.id}, '
      'blockType: ${node.getMetadataValue("blockType")}',
    );

    return DocumentHeaderComponentViewModel(
      nodeId: node.id,
      title: node.title,
      icon: node.icon,
      createdAt: DateTime.now(),
      // Note: Padding and maxWidth should be applied by the stylesheet through the SingleColumnLayoutViewModel
      // The component system will handle applying styles from the stylesheet
    );
  }
}

/// View model for DocumentHeaderComponent
class DocumentHeaderComponentViewModel
    extends SingleColumnLayoutComponentViewModel {
  DocumentHeaderComponentViewModel({
    required super.nodeId,
    required this.title,
    this.icon,
    super.padding = EdgeInsets.zero,
    super.maxWidth,
    required super.createdAt,
  });

  final AttributedText title;
  final String? icon;

  @override
  int get hashCode => Object.hash(nodeId, title, icon);

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! DocumentHeaderComponentViewModel) return false;
    if (nodeId != other.nodeId) return false;

    return title == other.title && icon == other.icon;
  }

  @override
  void applyStyles(Map<String, dynamic> styles) {
    super.applyStyles(styles);

    // Log when styles are applied to see if padding is coming through
    // tempoLog.info(
    //   '📐 DocumentHeader applyStyles called. Padding after applying: $padding, '
    //   'styles received: $styles',
    // );
  }

  @override
  DocumentHeaderComponentViewModel copy() {
    return DocumentHeaderComponentViewModel(
      nodeId: nodeId,
      title: title,
      icon: icon,
      padding: padding,
      maxWidth: maxWidth,
      createdAt: createdAt,
    );
  }
}
