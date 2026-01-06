import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../super_editor.dart';
import '../../ui/widgets/panel/panel_header.dart';
import '../../things/project/project.entity.dart';

class ProjectEditorHeader extends ConsumerStatefulWidget with PanelHeaderMixin {
  const ProjectEditorHeader(
    this.project,
    this._controller, {
    required this.focusNode,
    super.key,
  });

  factory ProjectEditorHeader.fromProject(ProjectEntity project) {
    final focusNode = FocusNode();
    final controller = AttributedTextEditingController();
    return ProjectEditorHeader(project, controller, focusNode: focusNode);
  }

  final ProjectEntity project;
  final FocusNode focusNode;
  final AttributedTextEditingController _controller;
  @override
  AttributedTextEditingController get controller => _controller;

  @override
  ConsumerState<ConsumerStatefulWidget> createState() =>
      _ProjectEditorHeaderState();
}

class _ProjectEditorHeaderState extends ConsumerState<ProjectEditorHeader> {
  @override
  Widget build(BuildContext context) {
    final hintText = widget.project.title;
    return PanelHeader(
      titleController: widget.controller,
      titleFocusNode: widget.focusNode,
      text: hintText,
      hintText: AttributedText("Give your project a title"),
      children: [],
    );
  }
}
