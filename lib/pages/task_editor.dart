import 'package:flutter/material.dart';
import 'package:tempo/app.dart' show obx;
import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/editor/super_text.dart';
import 'package:uuid/uuid.dart';
import 'package:tempo/src/database/objectbox/models/task.dart';
import 'package:tempo/src/database/objectbox/models/project.dart';
import 'package:tempo/utils/date_formatter.dart';
import 'package:tempo/src/infrastructure.dart';

class TaskEditorScreen extends StatefulWidget {
  final TaskEntity? task;
  final int? projectId; // Pre-selected project for new tasks

  const TaskEditorScreen({super.key, this.task, this.projectId});

  @override
  State<TaskEditorScreen> createState() => _TaskEditorScreenState();
}

class _TaskEditorScreenState extends State<TaskEditorScreen> {
  late AttributedTextEditingController _titleController;
  late FocusNode _titleFocusNode;
  late FocusNode _descriptionFocusNode;
  late DateTime _dueDate;
  late TaskStatus _status;
  TaskPriority _priority = TaskPriority.M;
  TaskTimeEstimate _timeEstimate = TaskTimeEstimate.M;
  TaskEnergyEstimate _energyEstimate = TaskEnergyEstimate.M;
  ProjectEntity? _selectedProject;
  List<ProjectEntity> _availableProjects = [];

  bool _isInitialized = false;

  @override
  void initState() {
    super.initState();
    _titleFocusNode = FocusNode(debugLabel: "task.editor.title");
    _descriptionFocusNode = FocusNode(debugLabel: "task.editor.description");

    _titleFocusNode.addListener(_onFocusChange);
    _descriptionFocusNode.addListener(_onFocusChange);

    _initializeEditor();
  }

  @override
  void dispose() {
    _titleFocusNode.removeListener(_onFocusChange);
    _descriptionFocusNode.removeListener(_onFocusChange);

    _titleFocusNode.dispose();
    _descriptionFocusNode.dispose();
    super.dispose();
  }

  void _onFocusChange() {
    // Force a repaint so that Container borders are correctly painted
    setState(() {});
  }

  void _initializeEditor() async {
    // Load projects
    _availableProjects = obx.getProjects();
    fluxEditorLog.finest("Loaded projects:");
    for (var project in _availableProjects) {
      fluxEditorLog.finest("- ${project.title}");
    }

    // Initialize with existing task data or defaults
    if (widget.task != null) {
      _titleController = AttributedTextEditingController(
        text: widget.task!.title,
      );
      _dueDate = widget.task!.dueDate;
      _status = widget.task!.status;
      _priority = widget.task!.priority;
      _timeEstimate = widget.task!.timeEstimate;
      _energyEstimate = widget.task!.energyEstimate;
      final currentProject = widget.task!.project.target;
      for (final project in _availableProjects) {
        if (currentProject != null && project == currentProject) {
          fluxEditorLog.finest("${project.id} =?= ${currentProject.id}");
          fluxEditorLog.finest("${project.uuid} =?= ${currentProject.uuid}");
          fluxEditorLog.finest("${project.title} =?= ${currentProject.title}");
          break;
        }
      }
      _selectedProject = widget.task!.project.target;
    } else {
      _titleController = AttributedTextEditingController();
      _dueDate = DateTime.now().add(const Duration(days: 1));
      _status = TaskStatus.todo;

      // If we have a pre-selected project, set it
      if (widget.projectId != null) {
        _selectedProject = _availableProjects.firstWhere(
          (p) => p.id == widget.projectId,
        );
      }
    }

    setState(() {
      _isInitialized = true;
    });
  }

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
        backgroundColor: Colors.transparent,
        elevation: 0,
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => Navigator.pop(context),
        ),
        actions: [
          TextButton.icon(
            icon: const Icon(Icons.save),
            label: const Text('Save'),
            onPressed: _saveTask,
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
                      value: _timeEstimate.toDisplayString(),
                      icon: Icons.timer_outlined,
                      color: _timeEstimate.color,
                      onTap: () => _showTimeEstimateSelector(),
                    ),
                    _buildPillButton(
                      label: 'Energy',
                      value: _energyEstimate.toDisplayString(),
                      icon: Icons.battery_charging_full_outlined,
                      color: _energyEstimate.color,
                      onTap: () => _showEnergyEstimateSelector(),
                    ),
                  ],
                ),
                const SizedBox(height: 16),

                // Due date
                _buildPillButton(
                  label: 'Due',
                  value: _formatRelativeDate(_dueDate),
                  icon: Icons.calendar_today_outlined,
                  onTap: _selectDueDate,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildTitleField() {
    return TempoTextField(
      focusNode: _titleFocusNode,
      controller: _titleController,
      hintText: AttributedText('Untitled'),
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

  String _formatRelativeDate(DateTime date) {
    final now = DateTime.now();
    final today = DateTime(now.year, now.month, now.day);
    final dateOnly = DateTime(date.year, date.month, date.day);
    
    final difference = dateOnly.difference(today).inDays;
    
    if (difference == 0) return 'Today';
    if (difference == 1) return 'Tomorrow';
    if (difference == -1) return 'Yesterday';
    if (difference > 1 && difference <= 6) return 'In $difference days';
    if (difference < -1 && difference >= -6) return '${-difference} days ago';
    if (difference >= 7 && difference < 14) return 'Next week';
    if (difference <= -7 && difference > -14) return 'Last week';
    
    // For dates further out, show month and day
    final months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
    final month = months[date.month - 1];
    
    // If it's the same year, just show month and day
    if (date.year == now.year) {
      return '$month ${date.day}';
    }
    
    // Otherwise show full date
    return '$month ${date.day}, ${date.year}';
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

  void _showTimeEstimateSelector() {
    _showEnumSelector<TaskTimeEstimate>(
      title: 'Select Time Estimate',
      values: TaskTimeEstimate.values,
      currentValue: _timeEstimate,
      getIcon: (time) => Icons.timer_outlined,
      getColor: (time) => time.color,
      onSelected: (time) {
        setState(() {
          _timeEstimate = time;
        });
      },
    );
  }

  void _showEnergyEstimateSelector() {
    _showEnumSelector<TaskEnergyEstimate>(
      title: 'Select Energy Estimate',
      values: TaskEnergyEstimate.values,
      currentValue: _energyEstimate,
      getIcon: (energy) => Icons.battery_charging_full_outlined,
      getColor: (energy) => energy.color,
      onSelected: (energy) {
        setState(() {
          _energyEstimate = energy;
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
        timeEstimate: _timeEstimate,
        energyEstimate: _energyEstimate,
      );

      // Set project if selected
      if (_selectedProject != null) {
        newTask.project.target = _selectedProject;
      }

      obx.putTask(newTask);
    } else {
      // Update existing task
      widget.task!.title = title;
      widget.task!.modifyDate = DateTime.now();
      widget.task!.dueDate = _dueDate;
      widget.task!.obxStatus = TaskStatus.values.indexOf(_status);
      widget.task!.obxPriority = TaskPriority.values.indexOf(_priority);
      widget.task!.obxTimeEstimate = TaskTimeEstimate.values.indexOf(
        _timeEstimate,
      );
      widget.task!.obxEnergyEstimate = TaskEnergyEstimate.values.indexOf(
        _energyEstimate,
      );

      // Update project association
      widget.task!.project.target = _selectedProject;

      obx.putTask(widget.task!);
    }

    Navigator.pop(context);
  }
}

Widget _buildTextField({
  required String label,
  required AttributedTextEditingController controller,
  required FocusNode focusNode,
  int minLines = 1,
  int maxLines = 1,
}) {
  return Container(
    padding: const EdgeInsets.all(8),
    decoration: BoxDecoration(
      border: Border.all(
        width: 1,
        color: focusNode.hasFocus ? Colors.blue : Colors.grey,
      ),
      borderRadius: BorderRadius.circular(4),
    ),
    child: TempoTextField(
      focusNode: focusNode,
      hintText: AttributedText(label),
      controller: controller,
      minLines: minLines,
      maxLines: maxLines,
      padding: const EdgeInsets.all(8),
    ),
  );
}
