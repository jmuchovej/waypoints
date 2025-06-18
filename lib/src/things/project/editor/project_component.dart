import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:super_editor/super_editor.dart';

import '../../../router.dart';
import '../project_entity.dart';
import '../../task/task_repository.dart';
import '../project_repository.dart';
import '../../../infrastructure/logging.dart';
import 'project_node.dart';

/// Component builder for ProjectNode
class ProjectComponentBuilder implements ComponentBuilder {
  const ProjectComponentBuilder();

  @override
  SingleColumnLayoutComponentViewModel? createViewModel(
    Document document,
    DocumentNode node,
  ) {
    if (node is! ProjectNode) {
      return null;
    }

    return ProjectComponentViewModel(nodeId: node.id, project: node.project);
  }

  @override
  Widget? createComponent(
    SingleColumnDocumentComponentContext componentContext,
    SingleColumnLayoutComponentViewModel componentViewModel,
  ) {
    if (componentViewModel is! ProjectComponentViewModel) {
      return null;
    }

    return ProjectComponent(
      componentKey: componentContext.componentKey,
      project: componentViewModel.project,
      // selection: componentViewModel.selection,
    );
  }
}

/// View model for the project component
class ProjectComponentViewModel extends SingleColumnLayoutComponentViewModel {
  ProjectComponentViewModel({
    required super.nodeId,
    required this.project,
    super.maxWidth,
    super.padding = EdgeInsets.zero,
  });

  final ProjectEntity project;

  @override
  ProjectComponentViewModel copy() {
    return ProjectComponentViewModel(
      nodeId: nodeId,
      project: project,
      maxWidth: maxWidth,
      padding: padding,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      super == other &&
          other is ProjectComponentViewModel &&
          runtimeType == other.runtimeType &&
          project.uuid == other.project.uuid;

  @override
  int get hashCode => super.hashCode ^ project.uuid.hashCode;
}

/// The visual component for displaying a project in the editor
class ProjectComponent extends ConsumerStatefulWidget {
  const ProjectComponent({
    super.key,
    required this.componentKey,
    required this.project,
    this.selection,
  });

  final GlobalKey componentKey;
  final ProjectEntity project;
  final DocumentSelection? selection;

  @override
  ConsumerState<ProjectComponent> createState() => _ProjectComponentState();
}

class _ProjectComponentState extends ConsumerState<ProjectComponent> {
  bool _isHovered = false;
  int? _taskCount;

  @override
  void initState() {
    super.initState();
    _loadTaskCount();
  }

  void _loadTaskCount() async {
    try {
      final taskRepo = ref.read(taskRepositoryProvider);
      final tasks = taskRepo.sameTypedChildren(widget.project.uuid);
      setState(() {
        _taskCount = tasks.length;
      });
    } catch (e) {
      fluxLog.warning('Failed to load task count for project: $e');
      setState(() {
        _taskCount = 0;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isSelected = widget.selection != null;

    return MouseRegion(
      key: widget.componentKey,
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: GestureDetector(
        onTap: () => _openProject(context),
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 150),
          margin: const EdgeInsets.symmetric(vertical: 4, horizontal: 8),
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color:
                isSelected
                    ? theme.colorScheme.primaryContainer.withValues(alpha: 0.3)
                    : _isHovered
                    ? theme.colorScheme.surfaceContainerHighest
                    : theme.colorScheme.surfaceContainer,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(
              color:
                  isSelected
                      ? theme.colorScheme.primary
                      : _isHovered
                      ? theme.colorScheme.outline.withValues(alpha: 0.3)
                      : Colors.transparent,
              width: isSelected ? 2 : 1,
            ),
            boxShadow:
                _isHovered
                    ? [
                      BoxShadow(
                        color: theme.shadowColor.withValues(alpha: 0.1),
                        blurRadius: 8,
                        offset: const Offset(0, 2),
                      ),
                    ]
                    : null,
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Header row with title and actions
              Row(
                children: [
                  // Project icon
                  Container(
                    padding: const EdgeInsets.all(8),
                    decoration: BoxDecoration(
                      color: theme.colorScheme.primaryContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Icon(
                      Icons.folder,
                      size: 20,
                      color: theme.colorScheme.onPrimaryContainer,
                    ),
                  ),
                  const SizedBox(width: 12),

                  // Project title
                  Expanded(
                    child: Text(
                      widget.project.title.toPlainText(),
                      style: theme.textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),

                  // Action buttons (visible on hover)
                  AnimatedOpacity(
                    duration: const Duration(milliseconds: 150),
                    opacity: _isHovered ? 1.0 : 0.0,
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        IconButton(
                          icon: const Icon(Icons.edit, size: 16),
                          onPressed: () => _editProject(context),
                          tooltip: 'Edit Project',
                          constraints: const BoxConstraints(
                            minWidth: 32,
                            minHeight: 32,
                          ),
                        ),
                        IconButton(
                          icon: const Icon(Icons.more_vert, size: 16),
                          onPressed: () => _showProjectMenu(context),
                          tooltip: 'More Options',
                          constraints: const BoxConstraints(
                            minWidth: 32,
                            minHeight: 32,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),

              const SizedBox(height: 12),

              // Project metadata
              Row(
                children: [
                  // Task count
                  if (_taskCount != null) ...[
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 4,
                      ),
                      decoration: BoxDecoration(
                        color: theme.colorScheme.surfaceContainerHighest,
                        borderRadius: BorderRadius.circular(12),
                      ),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(
                            Icons.task_alt,
                            size: 12,
                            color: theme.colorScheme.onSurfaceVariant,
                          ),
                          const SizedBox(width: 4),
                          Text(
                            '$_taskCount ${_taskCount == 1 ? 'task' : 'tasks'}',
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 8),
                  ],

                  // Modified date
                  Text(
                    _formatDate(widget.project.modifyDate),
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),

                  const Spacer(),

                  // Status indicators
                  if (widget.project.archived)
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 6,
                        vertical: 2,
                      ),
                      decoration: BoxDecoration(
                        color: theme.colorScheme.errorContainer,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Text(
                        'Archived',
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: theme.colorScheme.onErrorContainer,
                        ),
                      ),
                    ),
                ],
              ),

              // Project description (if available)
              // if (widget.project.notes != null &&
              //     widget.project.notes!.toPlainText().trim().isNotEmpty) ...[
              //   const SizedBox(height: 8),
              //   Text(
              //     widget.project.notes!.toPlainText(),
              //     style: theme.textTheme.bodySmall?.copyWith(
              //       color: theme.colorScheme.onSurfaceVariant,
              //     ),
              //     maxLines: 2,
              //     overflow: TextOverflow.ellipsis,
              //   ),
              // ],
            ],
          ),
        ),
      ),
    );
  }

  String _formatDate(DateTime date) {
    final now = DateTime.now();
    final difference = now.difference(date);

    if (difference.inDays == 0) {
      return 'Today';
    } else if (difference.inDays == 1) {
      return 'Yesterday';
    } else if (difference.inDays < 7) {
      return '${difference.inDays} days ago';
    } else if (difference.inDays < 30) {
      final weeks = (difference.inDays / 7).floor();
      return '$weeks ${weeks == 1 ? 'week' : 'weeks'} ago';
    } else {
      final months = (difference.inDays / 30).floor();
      return '$months ${months == 1 ? 'month' : 'months'} ago';
    }
  }

  void _openProject(BuildContext context) {
    context.gotoProject(widget.project);
  }

  void _editProject(BuildContext context) {
    // TODO: Navigate to project editor
    // context.router.push(ProjectEditorRoute(projectId: widget.project.uuid));
  }

  void _showProjectMenu(BuildContext context) {
    final theme = Theme.of(context);

    showModalBottomSheet(
      context: context,
      builder:
          (context) => Container(
            padding: const EdgeInsets.all(16),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  widget.project.title.toPlainText(),
                  style: theme.textTheme.titleMedium,
                ),
                const SizedBox(height: 16),
                ListTile(
                  leading: const Icon(Icons.edit),
                  title: const Text('Edit Project'),
                  onTap: () {
                    Navigator.pop(context);
                    _editProject(context);
                  },
                ),
                ListTile(
                  leading: Icon(
                    widget.project.archived ? Icons.unarchive : Icons.archive,
                  ),
                  title: Text(
                    widget.project.archived ? 'Unarchive' : 'Archive',
                  ),
                  onTap: () {
                    Navigator.pop(context);
                    _toggleArchive();
                  },
                ),
                const Divider(),
                ListTile(
                  leading: const Icon(Icons.delete, color: Colors.red),
                  title: const Text(
                    'Delete Project',
                    style: TextStyle(color: Colors.red),
                  ),
                  onTap: () {
                    Navigator.pop(context);
                    _confirmDelete(context);
                  },
                ),
              ],
            ),
          ),
    );
  }

  void _toggleArchive() {
    final projectRepo = ref.read(projectRepositoryProvider);
    projectRepo.archive(widget.project.uuid);
    _loadTaskCount(); // Refresh display
  }

  void _confirmDelete(BuildContext context) {
    showDialog(
      context: context,
      builder:
          (context) => AlertDialog(
            title: const Text('Delete Project'),
            content: Text(
              'Are you sure you want to delete "${widget.project.title.toPlainText()}"?\n\n'
              'Tasks in this project will not be deleted but will no longer be associated with the project.',
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context),
                child: const Text('Cancel'),
              ),
              TextButton(
                style: TextButton.styleFrom(foregroundColor: Colors.red),
                onPressed: () {
                  Navigator.pop(context);
                  _deleteProject();
                },
                child: const Text('Delete'),
              ),
            ],
          ),
    );
  }

  void _deleteProject() {
    final projectRepo = ref.read(projectRepositoryProvider);
    projectRepo.remove(widget.project.id);
  }
}
