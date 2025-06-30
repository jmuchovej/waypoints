import '../editor.dart';
import '../super_editor.dart';
import '../../infrastructure/logging.dart';

/// A generalized text node that can be either static or editable.
///
/// This node is designed to support various use cases like:
/// - Document headers (static or editable)
/// - Page titles (like Notion)
/// - Any single-line text that needs to toggle between view and edit modes
class EditableTextNode extends BlockNode {
  EditableTextNode({
    required this.id,
    required this.text,
    this.isEditable = true,
    this.placeholder = 'Untitled',
    this.textStyleKey = 'header1',
    this.icon,
    this.showIcon = true,
    this.maxLines = 1,
    this.onTextChanged,
    super.metadata,
  }) {
    initAddToMetadata({
      "blockType": NamedAttribution(EditableTextNode.blockType),
      "isEditable": isEditable,
      "textStyleKey": textStyleKey,
    });

    tempoLog.info(
      '📝 EditableTextNode created: id=$id, text="${text.toPlainText()}", '
      'isEditable=$isEditable, textStyleKey=$textStyleKey',
    );
  }

  /// Creates a document header node (replaces DocumentHeaderNode)
  static EditableTextNode documentHeader({
    required String id,
    required AttributedText title,
    String? icon,
    bool isEditable = false,
  }) {
    return EditableTextNode(
      id: id,
      text: title,
      isEditable: isEditable,
      textStyleKey: 'documentHeader',
      icon: icon,
      placeholder: 'Untitled Document',
    );
  }

  static const String blockType = 'editable_text';

  @override
  final String id;

  /// The text content
  final AttributedText text;

  /// Whether this text can be edited by the user
  final bool isEditable;

  /// Placeholder text when empty
  final String placeholder;

  /// Key to lookup text style from stylesheet
  final String textStyleKey;

  /// Optional icon identifier
  final String? icon;

  /// Whether to show the icon
  final bool showIcon;

  /// Maximum number of lines (1 for titles, more for descriptions)
  final int maxLines;

  /// Callback when text changes
  final void Function(AttributedText)? onTextChanged;

  @override
  bool hasEquivalentContent(DocumentNode other) {
    if (other is! EditableTextNode) return false;

    return text == other.text &&
        isEditable == other.isEditable &&
        textStyleKey == other.textStyleKey &&
        icon == other.icon;
  }

  EditableTextNode copyWith({
    String? id,
    AttributedText? text,
    bool? isEditable,
    String? placeholder,
    String? textStyleKey,
    String? icon,
    bool? showIcon,
    int? maxLines,
    void Function(AttributedText)? onTextChanged,
    Map<String, dynamic>? metadata,
  }) {
    return EditableTextNode(
      id: id ?? this.id,
      text: text ?? this.text,
      isEditable: isEditable ?? this.isEditable,
      placeholder: placeholder ?? this.placeholder,
      textStyleKey: textStyleKey ?? this.textStyleKey,
      icon: icon ?? this.icon,
      showIcon: showIcon ?? this.showIcon,
      maxLines: maxLines ?? this.maxLines,
      onTextChanged: onTextChanged ?? this.onTextChanged,
      metadata: metadata ?? this.metadata,
    );
  }

  @override
  DocumentNode copyAndReplaceMetadata(Map<String, dynamic> newMetadata) {
    return copyWith(metadata: newMetadata);
  }

  @override
  DocumentNode copyWithAddedMetadata(Map<String, dynamic> newProperties) {
    final newMetadata = Map<String, dynamic>.from(metadata);
    newMetadata.addAll(newProperties);
    return copyWith(metadata: newMetadata);
  }

  @override
  String? copyContent(NodeSelection selection) => text.toPlainText();

  @override
  int get hashCode =>
      Object.hash(id, text, isEditable, textStyleKey, icon, metadata);

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! EditableTextNode) return false;

    return id == other.id &&
        text == other.text &&
        isEditable == other.isEditable &&
        textStyleKey == other.textStyleKey &&
        icon == other.icon &&
        metadata == other.metadata;
  }
}

/// Factory for creating specific types of editable text nodes
class EditableTextNodeFactory {
  /// Creates a document header node (replaces DocumentHeaderNode)
  static EditableTextNode documentHeader({
    required String id,
    required AttributedText title,
    String? icon,
    bool isEditable = false,
  }) {
    return EditableTextNode(
      id: id,
      text: title,
      isEditable: isEditable,
      textStyleKey: 'documentHeader',
      icon: icon,
      placeholder: 'Untitled Document',
    );
  }

  /// Creates a page title node (like Notion)
  static EditableTextNode pageTitle({
    required String id,
    required AttributedText title,
    String? icon,
    void Function(AttributedText)? onTextChanged,
  }) {
    return EditableTextNode(
      id: id,
      text: title,
      isEditable: true,
      textStyleKey: 'pageTitle',
      icon: icon,
      placeholder: 'Untitled',
      onTextChanged: onTextChanged,
    );
  }

  /// Creates a section header
  static EditableTextNode sectionHeader({
    required String id,
    required AttributedText text,
    bool isEditable = true,
  }) {
    return EditableTextNode(
      id: id,
      text: text,
      isEditable: isEditable,
      textStyleKey: 'header2',
      showIcon: false,
      placeholder: 'Section Title',
    );
  }
}

/// Editable text specific commands
class ToggleEditableTextEditabilityCommand extends EditCommand {
  ToggleEditableTextEditabilityCommand({required this.nodeId});

  final String nodeId;

  @override
  HistoryBehavior get historyBehavior => HistoryBehavior.undoable;

  @override
  void execute(EditContext context, CommandExecutor executor) {
    final document = context.find<MutableDocument>(Editor.documentKey);
    final node = document.getNodeById(nodeId);

    if (node is! EditableTextNode) {
      tempoLog.warning(
        'Cannot toggle editability on non-EditableTextNode: ${node.runtimeType}',
      );
      return;
    }

    final newNode = node.copyWith(isEditable: !node.isEditable);
    document.replaceNodeById(node.id, newNode);
  }
}

/// Updates the text content of an EditableTextNode
class UpdateEditableTextCommand extends EditCommand {
  UpdateEditableTextCommand({required this.nodeId, required this.text});

  final String nodeId;
  final AttributedText text;

  @override
  HistoryBehavior get historyBehavior => HistoryBehavior.undoable;

  @override
  void execute(EditContext context, CommandExecutor executor) {
    final document = context.find<MutableDocument>(Editor.documentKey);
    final node = document.getNodeById(nodeId);

    if (node is! EditableTextNode) {
      tempoLog.warning(
        'Cannot update text on non-EditableTextNode: ${node.runtimeType}',
      );
      return;
    }

    final newNode = node.copyWith(text: text);
    document.replaceNodeById(node.id, newNode);
  }
}
