import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../objectbox/services/resolver_service.dart';
import '../../../things/project/project_entity.dart';
import '../../../things/task/task_entity.dart';

class ProjectCard extends ConsumerWidget {
  const ProjectCard({
    super.key,
    required this.project,
    required this.onTap,
    required this.onEdit,
    required this.onDelete,
  });

  final ProjectEntity project;
  final VoidCallback onTap;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final controller = AttributedTextEditingController(text: project.title);

    return Card(
      elevation: 2,
      child: InkWell(
        onTap: () {
          fluxLog.fine('ProjectCard tapped for: ${project.titlePlainText}');
          onTap();
        },
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(16.0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Project title
              TempoTextField(
                controller: controller,
                readOnly: true,
                textStyleBuilder: (attributions) {
                  return TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.w600,
                    color: theme.colorScheme.onSurface,
                  );
                },
              ),
              const Spacer(),
              // Project metadata
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  _buildMetadata(theme, ref),
                  _buildOptionsMenu(theme),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildMetadata(ThemeData theme, WidgetRef ref) {
    // Get children using the new polymorphic relationships
    final resolver = ref.read(resolverServiceProvider);
    final children = resolver.resolveMany(project.children.toList());
    final subProjects = children.whereType<ProjectEntity>().length;
    final tasks = children.whereType<TaskEntity>().length;

    return Row(
      children: [
        Icon(
          Icons.folder_outlined,
          size: 16,
          color: theme.colorScheme.onSurfaceVariant,
        ),
        const SizedBox(width: 4),
        Text(
          '$subProjects projects',
          style: TextStyle(
            fontSize: 14,
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(width: 12),
        Icon(
          Icons.task_alt,
          size: 16,
          color: theme.colorScheme.onSurfaceVariant,
        ),
        const SizedBox(width: 4),
        Text(
          '$tasks tasks',
          style: TextStyle(
            fontSize: 14,
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      ],
    );
  }

  Widget _buildOptionsMenu(ThemeData theme) {
    return PopupMenuButton<String>(
      icon: Icon(
        Icons.more_vert,
        size: 16,
        color: theme.colorScheme.onSurfaceVariant,
      ),
      itemBuilder:
          (context) => [
            const PopupMenuItem(value: 'edit', child: Text('Edit')),
            const PopupMenuItem(value: 'delete', child: Text('Delete')),
          ],
      onSelected: (value) {
        switch (value) {
          case 'edit':
            onEdit();
            break;
          case 'delete':
            onDelete();
            break;
        }
      },
    );
  }
}
