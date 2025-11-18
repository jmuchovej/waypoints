import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../editor/super_editor.dart';
import '../../objectbox/objectbox.g.dart';
import '../../things/project/project.entity.dart';
import '../../objectbox/repositories/project_repository.dart';
import '../../ui/panel/panel_wrapper.dart';
import '../../ui/widgets/panel/empty_state.dart';
import '../../ui/widgets/panel/panel_header.dart';

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
    final header = PanelHeader.readonly(title: AttributedText("Projects"));

    return PanelWrapper<ProjectEntity>(
      header: header,
      stream: getProjects(),
      emptyWidget: EmptyState(
        title: 'No projects found',
        subtitle: 'Create a new project by tapping the "+" button.',
        icon: Icons.task_alt_rounded,
        label: "Create Project",
        onPressed: () {},
      ),
      routePath: '/projects',
      actions: const [],
    );
  }

  Stream<ProjectEntity> getProjects() {
    final repo = ref.read(projectRepositoryProvider);

    final Condition<ProjectEntity> filter = repo.active();

    return repo.streamEntities(filter);
  }
}
