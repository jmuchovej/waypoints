import 'package:flutter/material.dart';
import 'package:tempo/app.dart' show obx;
import 'package:tempo/src/editor/super_editor.dart';
import 'package:uuid/uuid.dart';
import 'package:tempo/src/database/objectbox/models/task.dart';
import 'package:tempo/src/database/objectbox/models/project.dart';
import 'package:tempo/utils/date_formatter.dart';
import 'package:tempo/utils/logging.dart' show logger;

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
    _titleFocusNode = FocusNode();
    _descriptionFocusNode = FocusNode();

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
    logger.d("Loaded projects:");
    for (var project in _availableProjects) {
      logger.d("- ${project.title}");
    }

    // Initialize with existing task data or defaults
    if (widget.task != null) {
      _titleController = AttributedTextEditingController(
        text: AttributedText(widget.task!.title),
      );
      _dueDate = widget.task!.dueDate;
      _status = widget.task!.status;
      _priority = widget.task!.priority;
      _timeEstimate = widget.task!.timeEstimate;
      _energyEstimate = widget.task!.energyEstimate;
      final currentProject = widget.task!.project.target;
      for (final project in _availableProjects) {
        if (currentProject != null && project == currentProject) {
          logger.d("${project.id} =?= ${currentProject.id}");
          logger.d("${project.uuid} =?= ${currentProject.uuid}");
          logger.d("${project.title} =?= ${currentProject.title}");
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
        appBar: AppBar(
          title: Text(widget.task == null ? 'New Task' : 'Edit Task'),
        ),
        body: const Center(child: CircularProgressIndicator()),
      );
    }

    return Scaffold(
      appBar: AppBar(
        title: Text(widget.task == null ? 'New Task' : 'Edit Task'),
        actions: [
          IconButton(icon: const Icon(Icons.save), onPressed: _saveTask),
        ],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _buildTextField(
              label: "Task title",
              focusNode: _titleFocusNode,
              controller: _titleController,
            ),
            const SizedBox(height: 24),

            _buildRow([
              Expanded(
                child: _buildField(
                  'Due Date',
                  child: InkWell(
                    onTap: _selectDueDate,
                    child: InputDecorator(
                      decoration: const InputDecoration(
                        border: OutlineInputBorder(),
                        contentPadding: EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 8,
                        ),
                      ),
                      child: Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          Text(DateFormatter.formatDate(_dueDate)),
                          const Icon(Icons.calendar_today, size: 18),
                        ],
                      ),
                    ),
                  ),
                ),
              ),

              const SizedBox(width: 16),
              Expanded(
                child: _buildField(
                  'Project',
                  child: DropdownButtonFormField<ProjectEntity?>(
                    decoration: const InputDecoration(
                      border: OutlineInputBorder(),
                      contentPadding: EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                    ),
                    value: _selectedProject,
                    onChanged: (value) {
                      setState(() {
                        _selectedProject = value;
                      });
                    },
                    items:
                        _availableProjects.map((project) {
                          return DropdownMenuItem<ProjectEntity?>(
                            value: project,
                            child: Text(project.title),
                          );
                        }).toList(),
                  ),
                ),
              ),
            ]),
            const SizedBox(height: 16),

            _buildRow([
              Expanded(
                child: _buildField(
                  'Status',
                  child: DropdownButtonFormField<TaskStatus>(
                    decoration: const InputDecoration(
                      border: OutlineInputBorder(),
                      contentPadding: EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                    ),
                    value: _status,
                    onChanged: (TaskStatus? value) {
                      if (value != null) {
                        setState(() {
                          _status = value;
                        });
                      }
                    },
                    items:
                        TaskStatus.values
                            .map(
                              (s) => DropdownMenuItem<TaskStatus>(
                                value: s,
                                child: Text(s.toDisplayString()),
                              ),
                            )
                            .toList(),
                  ),
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: _buildField(
                  'Priority',
                  child: DropdownButtonFormField<TaskPriority>(
                    decoration: const InputDecoration(
                      border: OutlineInputBorder(),
                      contentPadding: EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                    ),
                    value: _priority,
                    onChanged: (TaskPriority? value) {
                      if (value != null) {
                        setState(() {
                          _priority = value;
                        });
                      }
                    },
                    items:
                        TaskPriority.values
                            .map(
                              (p) => DropdownMenuItem<TaskPriority>(
                                value: p,
                                child: Text(p.toDisplayString()),
                              ),
                            )
                            .toList(),
                  ),
                ),
              ),
            ]),
            const SizedBox(height: 16),

            _buildRow([
              Expanded(
                child: _buildField(
                  'Estimated Time',
                  child: DropdownButtonFormField<TaskTimeEstimate>(
                    decoration: const InputDecoration(
                      border: OutlineInputBorder(),
                      contentPadding: EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                    ),
                    value: _timeEstimate,
                    onChanged: (TaskTimeEstimate? value) {
                      if (value != null) {
                        setState(() {
                          _timeEstimate = value;
                        });
                      }
                    },
                    items:
                        TaskTimeEstimate.values
                            .map(
                              (t) => DropdownMenuItem<TaskTimeEstimate>(
                                value: t,
                                child: Text(t.toDisplayString()),
                              ),
                            )
                            .toList(),
                  ),
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: _buildField(
                  'Estimated Energy',
                  child: DropdownButtonFormField<TaskEnergyEstimate>(
                    decoration: const InputDecoration(
                      border: OutlineInputBorder(),
                      contentPadding: EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                    ),
                    value: _energyEstimate,
                    onChanged: (TaskEnergyEstimate? value) {
                      if (value != null) {
                        setState(() {
                          _energyEstimate = value;
                        });
                      }
                    },
                    items:
                        TaskEnergyEstimate.values
                            .map(
                              (e) => DropdownMenuItem<TaskEnergyEstimate>(
                                value: e,
                                child: Text(e.toDisplayString()),
                              ),
                            )
                            .toList(),
                  ),
                ),
              ),
            ]),

            const SizedBox(height: 32),
            SizedBox(
              width: double.infinity,
              height: 50,
              child: ElevatedButton(
                onPressed: _saveTask,
                child: Text(
                  widget.task == null ? 'Create Task' : 'Save Changes',
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildSectionTitle(String title) {
    return Text(
      title,
      style: const TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
    );
  }

  Widget _buildField(String label, {required Widget child}) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 14),
        ),
        const SizedBox(height: 4),
        child,
      ],
    );
  }

  Widget _buildRow(List<Widget> children) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: children,
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

    final title = _titleController.text.toPlainText().trim();

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
      widget.task!.obxTimeEstimate = TaskTimeEstimate.values.indexOf(_timeEstimate);
      widget.task!.obxEnergyEstimate = TaskEnergyEstimate.values.indexOf(_energyEstimate);

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
    child: SuperTextField(
      focusNode: focusNode,
      textController: controller,
      minLines: minLines,
      maxLines: maxLines,
      padding: const EdgeInsets.all(8),
      hintBuilder: (context) {
        return Text(label, style: TextStyle(color: Colors.grey, fontSize: 12));
      },
      hintBehavior: HintBehavior.displayHintUntilTextEntered,
    ),
  );
}
