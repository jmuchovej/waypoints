import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:jiffy/jiffy.dart';
import 'package:uuid/uuid.dart';

import '../../editor/super_editor.dart';
import '../../editor/super_text.dart';
import '../../infrastructure/extensions.dart';
import '../../router.dart';
import '../project/project_entity.dart';
import '../project/project_repository.dart';
import 'task_entity.dart';
import 'task_repository.dart';

class TaskEditor extends ConsumerStatefulWidget {
  const TaskEditor({
    super.key,
    this.task,
    this.projectID,
    this.showAppBar = true,
    this.panelInfo,
  });

  const TaskEditor.forPanel({super.key, required this.task, this.projectID, this.panelInfo})
    : showAppBar = false;

  final TaskEntity? task;
  final int? projectID; // Pre-selected project for new tasks
  final bool showAppBar; // Whether to show AppBar and Scaffold
  final dynamic panelInfo; // Panel context information (using dynamic for now to avoid import issues)

  @override
  ConsumerState<TaskEditor> createState() => _TaskEditorState();
}

class _TaskEditorState extends ConsumerState<TaskEditor> {
  late AttributedTextEditingController _titleController;
  late FocusNode _titleFocusNode;
  late FocusNode _descriptionFocusNode;
  late DateTime _dueDate;
  late TaskStatus _status;
  TaskPriority _priority = TaskPriority.M;
  TaskTime _time = TaskTime.M;
  TaskEnergy _energy = TaskEnergy.M;
  ProjectEntity? _selectedProject;
  List<ProjectEntity> _availableProjects = [];

  bool _isInitialized = false;

  @override
  Widget build(BuildContext context) {
    if (!_isInitialized) {
      final loadingWidget = const Center(child: CircularProgressIndicator());

      if (!widget.showAppBar) {
        return loadingWidget;
      }

      return Scaffold(
        backgroundColor: Theme.of(context).colorScheme.surface,
        body: loadingWidget,
      );
    }

    // Extract the main content with animation safety
    final bodyContent = LayoutBuilder(
      builder: (context, constraints) {
        // Wait for reasonable width before showing content to prevent overflow during panel opening animation
        if (constraints.maxWidth < 250) {
          return const Center(
            child: Padding(
              padding: EdgeInsets.all(20.0),
              child: CircularProgressIndicator(),
            ),
          );
        }
        
        // Normal content when width is sufficient
        return SingleChildScrollView(
          padding: const EdgeInsets.symmetric(horizontal: 40.0, vertical: 20.0),
          child: Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 800),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // Title row with status checkbox
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      // Status checkbox
                      Padding(
                        padding: const EdgeInsets.only(top: 8.0, right: 12.0),
                        child: _buildStatusCheckbox(),
                      ),
                      // Title field
                      Expanded(child: _buildTitleField()),
                    ],
                  ),
                  const SizedBox(height: 24),

                  // Properties row (Project, Priority, Time, Energy)
                  Wrap(
                    spacing: 12,
                    runSpacing: 12,
                    children: [
                      _buildPillButton(
                        label: 'Project',
                        value: _selectedProject?.title.toPlainText() ?? 'Project',
                        icon: Icons.folder_outlined,
                        hasValue: _selectedProject != null,
                        onTap: () => _showProjectSelector(),
                      ),
                      _buildPillButton(
                        label: 'Priority',
                        value: _priority.toDisplayString(),
                        icon: Icons.flag_outlined,
                        color: _priority.color,
                        onTap: () => _showPrioritySelector(),
                      ),
                      _buildPillButton(
                        label: 'Time',
                        value: _time.toDisplayString(),
                        icon: Icons.timer_outlined,
                        color: _time.color,
                        onTap: () => _showTimeEstimateSelector(),
                      ),
                      _buildPillButton(
                        label: 'Energy',
                        value: _energy.toDisplayString(),
                        icon: Icons.battery_charging_full_outlined,
                        color: _energy.color,
                        onTap: () => _showEnergyEstimateSelector(),
                      ),
                    ],
                  ),
                  const SizedBox(height: 16),

                  // Due date
                  _buildPillButton(
                    label: 'Due',
                    value: Jiffy.parseFromDateTime(_dueDate).fromNow(),
                    icon: Icons.calendar_today_outlined,
                    onTap: _selectDueDate,
                  ),
                ],
              ),
            ),
          ),
        );
      },
    );

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
              // If we can't pop, go to tasks page
              context.gotoTasks();
            }
          },
        ),
        actions: [
          TextButton.icon(
            icon: const Icon(Icons.save),
            label: const Text('Save'),
            onPressed: _saveTask,
          ),
        ],
      ),
      body: bodyContent,
    );
  }

  @override
  void dispose() {
    _titleFocusNode.removeListener(_onFocusChange);
    _descriptionFocusNode.removeListener(_onFocusChange);

    _titleFocusNode.dispose();
    _descriptionFocusNode.dispose();
    super.dispose();
  }

  @override
  void initState() {
    super.initState();
    _titleFocusNode = FocusNode(debugLabel: "task.editor.title");
    _descriptionFocusNode = FocusNode(debugLabel: "task.editor.description");

    _titleFocusNode.addListener(_onFocusChange);
    _descriptionFocusNode.addListener(_onFocusChange);

    _initializeEditor();
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

  Widget _buildStatusCheckbox() {
    return GestureDetector(
      onTap: () {
        setState(() {
          // Regular click: toggle between todo and complete
          _status =
              _status == TaskStatus.todo
                  ? TaskStatus.complete
                  : TaskStatus.todo;
        });
      },
      onSecondaryTap: () {
        // Right-click: show menu with dropped option
        _showStatusMenu();
      },
      onLongPress: () {
        // Long press on mobile: show menu with dropped option
        _showStatusMenu();
      },
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        child: Icon(_status.icon, size: 28, color: _status.color),
      ),
    );
  }

  Widget _buildTitleField() {
    return TempoTextField(
      focusNode: _titleFocusNode,
      controller: _titleController,
      hintText: AttributedText('Untitled'),
      textStyleBuilder: (attributions) {
        final theme = ColorScheme.of(context);
        return TextStyle(
          fontSize: 32,
          fontWeight: FontWeight.w600,
          height: 1.2,
          color: theme.onSurface,
        );
      },
      hintStyleBuilder: (attributions) {
        final theme = ColorScheme.of(context);
        return TextStyle(
          fontSize: 32,
          fontWeight: FontWeight.w600,
          height: 1.2,
          color: theme.onSurface.withValues(alpha: 0.3),
        );
      },
    );
  }

  void _initializeEditor() async {
    // Load projects
    final projectRepo = ref.read(projectRepositoryProvider);
    _availableProjects = projectRepo.getAll();

    // Initialize with existing task data or defaults
    if (widget.task != null) {
      _titleController = AttributedTextEditingController(
        text: widget.task!.title,
      );
      _dueDate = widget.task!.dueDate;
      _status = widget.task!.status;
      _priority = widget.task!.priority;
      _time = widget.task!.time;
      _energy = widget.task!.energy;
      _selectedProject = widget.task!.project.target;
    } else {
      _titleController = AttributedTextEditingController();
      _dueDate = DateTime.now().add(const Duration(days: 1));
      _status = TaskStatus.todo;

      // If we have a pre-selected project, set it
      if (widget.projectID != null) {
        _selectedProject = _availableProjects.firstWhere(
          (p) => p.id == widget.projectID,
        );
      }
    }

    setState(() {
      _isInitialized = true;
    });
  }

  void _onFocusChange() {
    // Force a repaint so that Container borders are correctly painted
    setState(() {});
  }

  void _saveTask() {
    if (_titleController.text.toPlainText().trim().isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Task title cannot be empty')),
      );
      return;
    }

    final title = _titleController.text;

    if (widget.task == null) {
      // Create new task
      final newTask = TaskEntity(
        uuid: const Uuid().v4(),
        title: title,
        createDate: DateTime.now(),
        modifyDate: DateTime.now(),
        dueDate: _dueDate,
        status: _status,
        priority: _priority,
        time: _time,
        energy: _energy,
      );

      // Set project if selected
      if (_selectedProject != null) {
        newTask.project.target = _selectedProject;
      }

      ref.read(taskRepositoryProvider).put(newTask);
    } else {
      // Update existing task
      widget.task!.title = title;
      widget.task!.modifyDate = DateTime.now();
      widget.task!.dueDate = _dueDate;
      widget.task!.statusOBX = TaskStatus.values.indexOf(_status);
      widget.task!.priorityOBX = TaskPriority.values.indexOf(_priority);
      widget.task!.timeOBX = TaskTime.values.indexOf(_time);
      widget.task!.energyOBX = TaskEnergy.values.indexOf(_energy);

      // Update project association
      widget.task!.project.target = _selectedProject;

      ref.read(taskRepositoryProvider).put(widget.task!);
    }

    if (context.router.canNavigateBack) {
      context.router.maybePop();
    } else {
      // If we can't pop, go to tasks page
      context.gotoTasks();
    }
  }

  Future<void> _selectDueDate() async {
    final DateTime? picked = await showDatePicker(
      context: context,
      initialDate: _dueDate,
      firstDate: DateTime.now().subtract(const Duration(days: 365)),
      lastDate: DateTime.now().add(const Duration(days: 365 * 5)),
    );

    if (picked != null && picked != _dueDate) {
      setState(() {
        _dueDate = picked;
      });
    }
  }

  void _showEnergyEstimateSelector() {
    _showEnumSelector<TaskEnergy>(
      title: 'Select Energy Estimate',
      values: TaskEnergy.values,
      currentValue: _energy,
      getIcon: (energy) => Icons.battery_charging_full_outlined,
      getColor: (energy) => energy.color,
      onSelected: (energy) {
        setState(() {
          _energy = energy;
        });
      },
    );
  }

  void _showEnumSelector<T extends Enum>({
    required String title,
    required List<T> values,
    required T currentValue,
    required IconData Function(T) getIcon,
    Color Function(T)? getColor,
    required void Function(T) onSelected,
  }) {
    showModalBottomSheet(
      context: context,
      builder:
          (context) => Container(
            padding: const EdgeInsets.all(16),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: TextTheme.of(context).titleLarge),
                const SizedBox(height: 16),
                ...values.map(
                  (value) => ListTile(
                    leading: Icon(getIcon(value), color: getColor?.call(value)),
                    title: Text(value.toString()),
                    selected: currentValue == value,
                    onTap: () {
                      onSelected(value);
                      context.pop();
                    },
                  ),
                ),
              ],
            ),
          ),
    );
  }

  void _showPrioritySelector() {
    _showEnumSelector<TaskPriority>(
      title: 'Select Priority',
      values: TaskPriority.values,
      currentValue: _priority,
      getIcon: (priority) => Icons.flag_outlined,
      getColor: (priority) => priority.color,
      onSelected: (priority) {
        setState(() {
          _priority = priority;
        });
      },
    );
  }

  void _showProjectSelector() {
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
                  'Select Project',
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                const SizedBox(height: 16),
                ..._availableProjects.map(
                  (project) => ListTile(
                    leading: const Icon(Icons.folder_outlined),
                    title: TempoTextField.fromAttributedText(
                      project.title,
                      readOnly: true,
                    ),
                    selected: _selectedProject == project,
                    onTap: () {
                      setState(() {
                        _selectedProject = project;
                      });
                      context.pop();
                    },
                  ),
                ),
                ListTile(
                  leading: const Icon(Icons.clear),
                  title: const Text('No project'),
                  selected: _selectedProject == null,
                  onTap: () {
                    setState(() {
                      _selectedProject = null;
                    });
                    Navigator.pop(context);
                  },
                ),
              ],
            ),
          ),
    );
  }

  void _showStatusMenu() {
    final RenderBox button = context.findRenderObject() as RenderBox;
    final RenderBox overlay =
        Navigator.of(context).overlay!.context.findRenderObject() as RenderBox;
    final Offset position = button.localToGlobal(
      Offset.zero,
      ancestor: overlay,
    );

    showMenu<TaskStatus>(
      context: context,
      position: RelativeRect.fromLTRB(
        position.dx,
        position.dy + button.size.height,
        position.dx + button.size.width,
        position.dy,
      ),
      items: [
        PopupMenuItem(
          value: TaskStatus.todo,
          child: Row(
            children: [
              Icon(TaskStatus.todo.icon, color: TaskStatus.todo.color),
              const SizedBox(width: 8),
              Text(TaskStatus.todo.toDisplayString()),
            ],
          ),
        ),
        PopupMenuItem(
          value: TaskStatus.complete,
          child: Row(
            children: [
              Icon(TaskStatus.complete.icon, color: TaskStatus.complete.color),
              const SizedBox(width: 8),
              Text(TaskStatus.complete.toDisplayString()),
            ],
          ),
        ),
        const PopupMenuDivider(),
        PopupMenuItem(
          value: TaskStatus.dropped,
          child: Row(
            children: [
              Icon(TaskStatus.dropped.icon, color: TaskStatus.dropped.color),
              const SizedBox(width: 8),
              Text(TaskStatus.dropped.toDisplayString()),
            ],
          ),
        ),
      ],
    ).then((value) {
      if (value != null) {
        setState(() {
          _status = value;
        });
      }
    });
  }

  void _showTimeEstimateSelector() {
    _showEnumSelector<TaskTime>(
      title: 'Select Time Estimate',
      values: TaskTime.values,
      currentValue: _time,
      getIcon: (time) => Icons.timer_outlined,
      getColor: (time) => time.color,
      onSelected: (time) {
        setState(() {
          _time = time;
        });
      },
    );
  }
}
