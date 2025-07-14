import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../super_editor.dart';
import '../../ui/widgets/panel/panel_header.dart';
import '../../things/project/project_entity.dart';

class ProjectEditorHeader extends ConsumerStatefulWidget {
  const ProjectEditorHeader(this.project, {
    required this.focusNode,
    required this.controller,
    super.key,
  });

  factory ProjectEditorHeader.fromProject(ProjectEntity project) {
    final focusNode = FocusNode();
    final controller = AttributedTextEditingController();
    return ProjectEditorHeader(project, focusNode: focusNode, controller: controller);
  }

  final ProjectEntity project;
  final FocusNode focusNode;
  final AttributedTextEditingController controller;

  @override
  ConsumerState<ConsumerStatefulWidget> createState() => _ProjectEditorHeaderState();
}


class _ProjectEditorHeaderState extends ConsumerState<ProjectEditorHeader> {
  @override
  Widget build(BuildContext context) {
    final hintText = AttributedText("New Project");
    return PanelHeader(
      titleController: widget.controller,
      titleFocusNode: widget.focusNode,
      hintText: hintText,
      children: []
    );
  }

}
