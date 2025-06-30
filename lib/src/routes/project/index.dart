import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../editor/nodes/editable_text_node.dart';
import '../../editor/super_editor.dart';
import '../../objectbox/objectbox.g.dart';
import '../../things/project/project_entity.dart';
import '../../things/project/project_repository.dart';
import '../../ui/panel/panel_preconfigured.dart';
import '../../ui/widgets/panel/empty_state.dart';

final EditableTextNode header = EditableTextNode.documentHeader(
  id: "projects-header",
  title: AttributedText("Projects"),
  isEditable: false,
);

@RoutePage(name: 'ProjectsRoute')
class ProjectsScreen extends ConsumerStatefulWidget {
  const ProjectsScreen({super.key});

  @override
  ConsumerState<ConsumerStatefulWidget> createState() => _ProjectsScreenState();
}

class _ProjectsScreenState extends ConsumerState<ProjectsScreen> {
  @override
  void initState() {
    super.initState();
  }

  @override
  void dispose() {
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return PreconfiguredPanel<ProjectEntity>(
      header: header,
      stream: getProjects(),
      emptyWidget: EmptyState(
        title: 'No projects found',
        subtitle: 'Create a new project by tapping the "+" button.',
        icon: Icons.task_alt_rounded,
        label: "Create Project",
        onPressed: () {},
      ),
      actions: const [],
    );
  }

  Stream<ProjectEntity> getProjects() {
    final repo = ref.read(projectRepositoryProvider);

    final Condition<ProjectEntity> filter = repo.active();

    return repo.streamEntities(filter);
  }
}
