import 'package:flutter/material.dart';
import '../../../things/project/project_entity.dart';
import 'project_card.dart';

class ProjectGrid extends StatelessWidget {
  const ProjectGrid({
    super.key,
    required this.projects,
    required this.onProjectTap,
    required this.onProjectEdit,
    required this.onProjectDelete,
  });

  final List<ProjectEntity> projects;
  final void Function(ProjectEntity) onProjectTap;
  final void Function(ProjectEntity) onProjectEdit;
  final void Function(ProjectEntity) onProjectDelete;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 1200),
        child: Padding(
          padding: const EdgeInsets.all(24.0),
          child: GridView.builder(
            gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
              crossAxisCount: _calculateCrossAxisCount(context),
              crossAxisSpacing: 16.0,
              mainAxisSpacing: 16.0,
              childAspectRatio: 1.2,
            ),
            itemCount: projects.length,
            itemBuilder: (context, index) {
              final project = projects[index];
              return ProjectCard(
                project: project,
                onTap: () => onProjectTap(project),
                onEdit: () => onProjectEdit(project),
                onDelete: () => onProjectDelete(project),
              );
            },
          ),
        ),
      ),
    );
  }

  int _calculateCrossAxisCount(BuildContext context) {
    final screenWidth = MediaQuery.of(context).size.width;
    if (screenWidth > 1200) return 4;
    if (screenWidth > 800) return 3;
    if (screenWidth > 600) return 2;
    return 1;
  }
}
