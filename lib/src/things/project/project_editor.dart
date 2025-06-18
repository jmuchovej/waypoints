import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:jiffy/jiffy.dart';
import 'package:uuid/uuid.dart';

import '../../editor/super_editor.dart';
import '../../editor/super_text.dart';
import '../../router.dart';
import '../task/task_entity.dart';
import '../task/task_repository.dart';
import 'project_entity.dart';
import 'project_repository.dart';

/// Project editor similar to TaskEditor but for project management
class ProjectEditor extends ConsumerStatefulWidget {
  const ProjectEditor({super.key, this.project});

  final ProjectEntity? project;

  @override
  ConsumerState<ProjectEditor> createState() => _ProjectEditorState();
}

class _ProjectEditorState extends ConsumerState<ProjectEditor> {
  late AttributedTextEditingController _titleController;
  late AttributedTextEditingController _notesController;
  late FocusNode _titleFocusNode;
  late FocusNode _notesFocusNode;
  ProjectEntity? _parentProject;
  List<ProjectEntity> _availableParents = [];
  List<TaskEntity> _projectTasks = [];

  bool _isInitialized = false;

  @override
  Widget build(BuildContext context) {
    if (!_isInitialized) {
      return Scaffold(
        backgroundColor: Theme.of(context).colorScheme.surface,
        body: const Center(child: CircularProgressIndicator()),
      );
    }

    return Scaffold(
      backgroundColor: Theme.of(context).colorScheme.surface,
      appBar: AppBar(
        backgroundColor: Theme.of(context).colorScheme.surface,
        elevation: 0,
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () {
            if (context.router.canNavigateBack) {
              context.router.maybePop();
            } else {
              context.gotoProjects();
            }
          },
        ),
        actions: [
          TextButton.icon(
            icon: const Icon(Icons.save),
            label: const Text('Save'),
            onPressed: _saveProject,
          ),
        ],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.symmetric(horizontal: 40.0, vertical: 20.0),
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 800),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Title field with project icon
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // Project icon
                    Padding(
                      padding: const EdgeInsets.only(top: 8.0, right: 12.0),
                      child: Container(
                        padding: const EdgeInsets.all(8),
                        decoration: BoxDecoration(
                          color: Theme.of(context).colorScheme.primaryContainer,
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: Icon(
                          Icons.folder,
                          size: 24,
                          color:
                              Theme.of(context).colorScheme.onPrimaryContainer,
                        ),
                      ),
                    ),
                    // Title field
                    Expanded(child: _buildTitleField()),
                  ],
                ),
                const SizedBox(height: 24),

                // Properties row
                Wrap(
                  spacing: 12,
                  runSpacing: 12,
                  children: [
                    _buildPillButton(
                      label: 'Parent Project',
                      value: _parentProject?.title.toPlainText() ?? 'None',
                      icon: Icons.folder_outlined,
                      hasValue: _parentProject != null,
                      onTap: () => _showParentProjectSelector(),
                    ),
                    _buildInfoPill(
                      label: 'Tasks',
                      value: '${_projectTasks.length}',
                      icon: Icons.task_alt,
                    ),
                    if (widget.project != null) ...[
                      _buildInfoPill(
                        label: 'Created',
                        value:
                            Jiffy.parseFromDateTime(
                              widget.project!.createDate,
                            ).fromNow(),
                        icon: Icons.access_time,
                      ),
                      _buildInfoPill(
                        label: 'Modified',
                        value:
                            Jiffy.parseFromDateTime(
                              widget.project!.modifyDate,
                            ).fromNow(),
                        icon: Icons.edit,
                      ),
                    ],
                  ],
                ),
                const SizedBox(height: 32),

                // Notes section
                Text(
                  'Notes',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 8),
                _buildNotesField(),

                // Tasks section (if editing existing project)
                if (widget.project != null) ...[
                  const SizedBox(height: 32),
                  _buildTasksSection(),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }

  @override
  void dispose() {
    _titleFocusNode.removeListener(_onFocusChange);
    _notesFocusNode.removeListener(_onFocusChange);

    _titleFocusNode.dispose();
    _notesFocusNode.dispose();
    _titleController.dispose();
    _notesController.dispose();
    super.dispose();
  }

  @override
  void initState() {
    super.initState();
    _titleFocusNode = FocusNode(debugLabel: "project.editor.title");
    _notesFocusNode = FocusNode(debugLabel: "project.editor.notes");

    _titleFocusNode.addListener(_onFocusChange);
    _notesFocusNode.addListener(_onFocusChange);

    _initializeEditor();
  }

  Widget _buildInfoPill({
    required String label,
    required String value,
    required IconData icon,
    Color? color,
  }) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(20),
        border: Border.all(
          color: theme.colorScheme.outline.withValues(alpha: 0.1),
        ),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            icon,
            size: 16,
            color: color ?? theme.colorScheme.onSurfaceVariant,
          ),
          const SizedBox(width: 6),
          Text(
            value,
            style: TextStyle(
              fontSize: 12,
              color: color ?? theme.colorScheme.onSurface,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildNotesField() {
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(
          color:
              _notesFocusNode.hasFocus
                  ? Theme.of(context).colorScheme.primary
                  : Theme.of(
                    context,
                  ).colorScheme.outline.withValues(alpha: 0.2),
        ),
        borderRadius: BorderRadius.circular(8),
      ),
      child: TempoTextField(
        focusNode: _notesFocusNode,
        controller: _notesController,
        hintText: AttributedText('Add project notes...'),
        minLines: 4,
        maxLines: 10,
        padding: const EdgeInsets.all(16),
        textStyleBuilder: (attributions) {
          return Theme.of(context).textTheme.bodyMedium!;
        },
        hintStyleBuilder: (attributions) {
          return Theme.of(context).textTheme.bodyMedium!.copyWith(
            color: Theme.of(
              context,
            ).colorScheme.onSurface.withValues(alpha: 0.5),
          );
        },
      ),
    );
  }

  Widget _buildPillButton({
    required String label,
    required String value,
    required IconData icon,
    Color? color,
    required VoidCallback onTap,
    bool hasValue = true,
  }) {
    final theme = Theme.of(context);
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(20),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: BoxDecoration(
          color: theme.colorScheme.surfaceContainerHighest,
          borderRadius: BorderRadius.circular(20),
          border: Border.all(
            color: theme.colorScheme.outline.withValues(alpha: 0.2),
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              icon,
              size: 16,
              color: color ?? theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(width: 6),
            if (hasValue) ...[
              Text(
                value,
                style: TextStyle(
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                  color: color ?? theme.colorScheme.onSurface,
                ),
              ),
            ] else ...[
              Text(
                label,
                style: TextStyle(
                  fontSize: 12,
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildTaskItem(TaskEntity task) {
    final theme = Theme.of(context);

    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        border: Border(
          bottom: BorderSide(
            color: theme.colorScheme.outline.withValues(alpha: 0.1),
            width: 1,
          ),
        ),
      ),
      child: Row(
        children: [
          Icon(task.status.icon, size: 16, color: task.status.color),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              task.title.toPlainText(),
              style: theme.textTheme.bodyMedium,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          const SizedBox(width: 8),
          Text(
            Jiffy.parseFromDateTime(task.dueDate).fromNow(),
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildTasksSection() {
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text(
              'Tasks (${_projectTasks.length})',
              style: theme.textTheme.titleMedium?.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            const Spacer(),
            TextButton.icon(
              icon: const Icon(Icons.add, size: 16),
              label: const Text('Add Task'),
              onPressed: () => _createTaskInProject(),
            ),
          ],
        ),
        const SizedBox(height: 8),

        if (_projectTasks.isEmpty) ...[
          Container(
            padding: const EdgeInsets.all(24),
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: Center(
              child: Column(
                children: [
                  Icon(
                    Icons.task_alt,
                    size: 48,
                    color: theme.colorScheme.onSurfaceVariant.withValues(
                      alpha: 0.6,
                    ),
                  ),
                  const SizedBox(height: 8),
                  Text('No tasks yet', style: theme.textTheme.titleSmall),
                  const SizedBox(height: 4),
                  Text(
                    'Add tasks to organize work in this project',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ] else ...[
          // Task list
          DecoratedBox(
            decoration: BoxDecoration(
              border: Border.all(
                color: theme.colorScheme.outline.withValues(alpha: 0.2),
              ),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Column(
              children: [
                ..._projectTasks.take(5).map((task) => _buildTaskItem(task)),
                if (_projectTasks.length > 5)
                  Container(
                    padding: const EdgeInsets.all(16),
                    child: Text(
                      'and ${_projectTasks.length - 5} more tasks...',
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ],
      ],
    );
  }

  Widget _buildTitleField() {
    return TempoTextField(
      focusNode: _titleFocusNode,
      controller: _titleController,
      hintText: AttributedText('Untitled Project'),
      textStyleBuilder: (attributions) {
        final theme = Theme.of(context);
        return TextStyle(
          fontSize: 32,
          fontWeight: FontWeight.w600,
          height: 1.2,
          color: theme.colorScheme.onSurface,
        );
      },
      hintStyleBuilder: (attributions) {
        final theme = Theme.of(context);
        return TextStyle(
          fontSize: 32,
          fontWeight: FontWeight.w600,
          height: 1.2,
          color: theme.colorScheme.onSurface.withValues(alpha: 0.3),
        );
      },
    );
  }

  void _createTaskInProject() {
    // Navigate to task creation with pre-selected project
    context.router.push(CreateTaskRoute(projectId: widget.project?.id));
  }

  void _initializeEditor() async {
    // Load available parent projects
    final projectRepo = ref.read(projectRepositoryProvider);
    _availableParents = projectRepo.active();

    // Remove current project from available parents (can't be parent of itself)
    if (widget.project != null) {
      _availableParents.removeWhere((p) => p.uuid == widget.project!.uuid);
    }

    // Initialize with existing project data or defaults
    if (widget.project != null) {
      _titleController = AttributedTextEditingController(
        text: widget.project!.title,
      );
      _notesController = AttributedTextEditingController(
        text: AttributedText(''),
      );
      _parentProject = _availableParents.firstWhereOrNull(
        (p) => p.uuid == widget.project!.parent.target?.uuid,
      );

      // Load tasks in this project
      final taskRepo = ref.read(taskRepositoryProvider);
      _projectTasks = taskRepo.sameTypedChildren(widget.project!.uuid);
    } else {
      _titleController = AttributedTextEditingController();
      _notesController = AttributedTextEditingController();
      _projectTasks = [];
    }

    setState(() {
      _isInitialized = true;
    });
  }

  void _onFocusChange() {
    // Force a repaint so that Container borders are correctly painted
    setState(() {});
  }

  void _saveProject() {
    if (_titleController.text.toPlainText().trim().isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Project title cannot be empty')),
      );
      return;
    }

    final title = _titleController.text;

    if (widget.project == null) {
      // Create new project
      final newProject = ProjectEntity(
        uuid: const Uuid().v4(),
        title: title,
        createDate: DateTime.now(),
        modifyDate: DateTime.now(),
      );
      newProject.parent.target = _parentProject;

      ref.read(projectRepositoryProvider).put(newProject);
    } else {
      // Update existing project
      widget.project!.title = title;
      // widget.project!.notes = notes;
      widget.project!.modifyDate = DateTime.now();
      widget.project!.parent.target = _parentProject;

      ref.read(projectRepositoryProvider).put(widget.project!);
    }

    if (context.router.canNavigateBack) {
      context.router.maybePop();
    } else {
      context.gotoProjects();
    }
  }

  void _showParentProjectSelector() {
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
                  'Select Parent Project',
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                const SizedBox(height: 16),
                ..._availableParents.map(
                  (project) => ListTile(
                    leading: const Icon(Icons.folder_outlined),
                    title: Text(project.title.toPlainText()),
                    selected: _parentProject == project,
                    onTap: () {
                      setState(() {
                        _parentProject = project;
                      });
                      Navigator.pop(context);
                    },
                  ),
                ),
                ListTile(
                  leading: const Icon(Icons.clear),
                  title: const Text('No parent project'),
                  selected: _parentProject == null,
                  onTap: () {
                    setState(() {
                      _parentProject = null;
                    });
                    Navigator.pop(context);
                  },
                ),
              ],
            ),
          ),
    );
  }
}

extension _FirstWhereOrNull<T> on List<T> {
  T? firstWhereOrNull(bool Function(T) test) {
    for (final element in this) {
      if (test(element)) return element;
    }
    return null;
  }
}
