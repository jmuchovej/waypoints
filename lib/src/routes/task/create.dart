import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:jiffy/jiffy.dart';
import 'package:uuid/uuid.dart';
import 'package:go_router/go_router.dart';

import '../../editor/editor.dart';
import '../../infrastructure/extensions.dart';
import '../../things/project/project_entity.dart';
import '../../things/project/project_repository.dart';
import '../../things/task/task_entity.dart';
import '../../things/task/task_repository.dart';

class CreateTaskScreen extends ConsumerStatefulWidget {
  const CreateTaskScreen({super.key, this.projectId});

  final int? projectId; // Pre-selected project for new tasks

  @override
  ConsumerState<CreateTaskScreen> createState() => _CreateTaskScreenState();
}

class _CreateTaskScreenState extends ConsumerState<CreateTaskScreen> {
  late AttributedTextEditingController _titleController;
  late FocusNode _titleFocusNode;
  late DateTime _dueDate;
  TaskStatus _status = TaskStatus.todo;
  TaskPriority _priority = TaskPriority.M;
  TaskTime _time = TaskTime.M;
  TaskEnergy _energy = TaskEnergy.M;
  ProjectEntity? _selectedProject;
  List<ProjectEntity> _availableProjects = [];
  bool _isInitialized = false;

  @override
  Widget build(BuildContext context) {
    if (!_isInitialized) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }

    final theme = Theme.of(context);

    return Scaffold(
      backgroundColor: theme.colorScheme.surface,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        elevation: 0,
        leading: IconButton(
          icon: const Icon(Icons.close),
          onPressed: () {
            if (context.canPop()) {
              context.pop();
            } else {
              context.go('/tasks');
            }
          },
        ),
        actions: [
          TextButton.icon(
            icon: const Icon(Icons.save),
            label: const Text('Create'),
            onPressed: _createTask,
          ),
        ],
      ),
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 800),
          child: SingleChildScrollView(
            padding: const EdgeInsets.symmetric(
              horizontal: 24.0,
              vertical: 20.0,
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Title section
                Text(
                  'New Task',
                  style: theme.textTheme.displayMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 32),

                // Task title
                _buildTitleField(theme),
                const SizedBox(height: 24),

                // Properties
                _buildPropertiesSection(theme),
              ],
            ),
          ),
        ),
      ),
    );
  }

  @override
  void dispose() {
    _titleFocusNode.dispose();
    _titleController.dispose();
    super.dispose();
  }

  @override
  void initState() {
    super.initState();
    _titleFocusNode = FocusNode(debugLabel: "task.create.title");
    _titleController = AttributedTextEditingController();
    _dueDate = DateTime.now().add(const Duration(days: 1));
    _initializeData();
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

  Widget _buildPropertiesSection(ThemeData theme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Properties',
          style: theme.textTheme.titleMedium?.copyWith(
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: 16),

        // Properties row
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
    );
  }

  Widget _buildTitleField(ThemeData theme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Task Title',
          style: theme.textTheme.titleMedium?.copyWith(
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: 8),
        TempoTextField(
          focusNode: _titleFocusNode,
          controller: _titleController,
          hintText: AttributedText('What needs to be done?'),
          textStyleBuilder: (attributions) {
            return TextStyle(fontSize: 18, color: theme.colorScheme.onSurface);
          },
          hintStyleBuilder: (attributions) {
            return TextStyle(
              fontSize: 18,
              color: theme.colorScheme.onSurface.withValues(alpha: 0.4),
            );
          },
        ),
      ],
    );
  }

  void _createTask() {
    if (_titleController.text.toPlainText().trim().isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Task title cannot be empty')),
      );
      return;
    }

    final title = _titleController.text;

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

    // Navigate back
    if (context.canPop()) {
      context.pop();
    } else {
      context.go('/tasks');
    }
  }

  void _initializeData() async {
    // Load projects
    _availableProjects = ref.read(projectRepositoryProvider).getAll();

    // If we have a pre-selected project, set it
    if (widget.projectId != null) {
      _selectedProject = _availableProjects.firstWhere(
        (p) => p.id == widget.projectId,
        orElse: () => _availableProjects.first,
      );
    }

    setState(() {
      _isInitialized = true;
    });
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
                Text(title, style: Theme.of(context).textTheme.titleLarge),
                const SizedBox(height: 16),
                ...values.map(
                  (value) => ListTile(
                    leading: Icon(getIcon(value), color: getColor?.call(value)),
                    title: Text(value.toDisplayString()),
                    selected: currentValue == value,
                    onTap: () {
                      onSelected(value);
                      Navigator.pop(context);
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
                      Navigator.pop(context);
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
