import 'dart:developer';

import 'package:flutter/material.dart';
import 'package:flux/models/task/status.dart';
import 'package:flux/models/task/priority.dart';
import 'package:flux/models/task/energy.dart';
import 'package:flux/models/task/time.dart';
import 'package:super_editor/super_editor.dart';
import 'package:uuid/uuid.dart';
import '../../models/task.dart' as Task;
import '../../models/task_list.dart' show TaskList;
import '../../objectbox.dart';
import '../../utils/date_formatter.dart';
import '../../utils/extensions.dart';
import '../../utils/logging.dart' show logger;

class TaskEditorScreen extends StatefulWidget {
  final Task.Task? task;
  final int? projectId; // Pre-selected project for new tasks

  const TaskEditorScreen({Key? key, this.task, this.projectId})
    : super(key: key);

  @override
  _TaskEditorScreenState createState() => _TaskEditorScreenState();
}

class _TaskEditorScreenState extends State<TaskEditorScreen> {
  late AttributedTextEditingController _nameController;
  late FocusNode _nameFocusNode;
  late AttributedTextEditingController _descriptionController;
  late FocusNode _descriptionFocusNode;
  late DateTime _dueDate;
  late Status _status;
  Priority _priority = Priority.unknown;
  EstimatedTime _estimatedTime = EstimatedTime.unknown;
  EstimatedEnergy _estimatedEnergy = EstimatedEnergy.unknown;
  TaskList? _selectedProject;
  List<TaskList> _availableProjects = [];

  bool _isInitialized = false;

  @override
  void initState() {
    super.initState();
    _nameFocusNode = FocusNode();
    _descriptionFocusNode = FocusNode();

    _nameFocusNode.addListener(_onFocusChange);
    _descriptionFocusNode.addListener(_onFocusChange);

    _initializeEditor();
  }

  @override
  void dispose() {
    _nameFocusNode.removeListener(_onFocusChange);
    _descriptionFocusNode.removeListener(_onFocusChange);

    _nameFocusNode.dispose();
    _descriptionFocusNode.dispose();
    super.dispose();
  }

  void _onFocusChange() {
    // Force a repaint so that Container borders are correctly painted
    setState(() {});
  }

  void _initializeEditor() async {
    // Load projects
    _availableProjects = ObjectBox.taskListBox().getAll();
    logger.d("Loaded projects:");
    for (var project in _availableProjects) {
      logger.d("- ${project.name}");
    }

    // Initialize with existing task data or defaults
    if (widget.task != null) {
      _nameController = AttributedTextEditingController(
        text: AttributedText(widget.task!.name),
      );
      _dueDate = widget.task!.dueDate;
      _status = widget.task!.status;
      _priority = widget.task!.priority;
      _estimatedTime = widget.task!.estimatedTime;
      _estimatedEnergy = widget.task!.estimatedEnergy;
      var taskList = widget.task!.taskList.target;
      for (var project in _availableProjects) {
        if (taskList != null && project.id == taskList.id) {
          logger.d("${project.id} =?= ${taskList.id}");
          logger.d("${project.uuid} =?= ${taskList.uuid}");
          logger.d("${project.name} =?= ${taskList.name}");
          logger.d("${project.description} =?= ${taskList.description}");
          break;
        }
      }
      _selectedProject = widget.task!.taskList.target;

      // Initialize SuperEditor document with task description
      _descriptionController = AttributedTextEditingController(
        text: AttributedText(widget.task!.description),
      );
    } else {
      _nameController = AttributedTextEditingController();
      _descriptionController = AttributedTextEditingController();
      _dueDate = DateTime.now().add(const Duration(days: 1));
      _status = Status.todo;

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
              label: "Task Name",
              focusNode: _nameFocusNode,
              controller: _nameController,
            ),
            const SizedBox(height: 24),

            _buildTextField(
              label: "Description",
              focusNode: _descriptionFocusNode,
              controller: _descriptionController,
              minLines: 3,
              maxLines: 5,
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
                  child: DropdownButtonFormField<TaskList?>(
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
                          return DropdownMenuItem<TaskList?>(
                            value: project,
                            child: Text(project.name),
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
                  child: DropdownButtonFormField<Status>(
                    decoration: const InputDecoration(
                      border: OutlineInputBorder(),
                      contentPadding: EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                    ),
                    value: _status,
                    onChanged: (Status? value) {
                      if (value != null) {
                        setState(() {
                          _status = value;
                        });
                      }
                    },
                    items:
                        Status.values
                            .where((s) => s != Status.unknown)
                            .map(
                              (s) => DropdownMenuItem<Status>(
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
                  child: DropdownButtonFormField<Priority>(
                    decoration: const InputDecoration(
                      border: OutlineInputBorder(),
                      contentPadding: EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                    ),
                    value: _priority,
                    onChanged: (Priority? value) {
                      if (value != null) {
                        setState(() {
                          _priority = value;
                        });
                      }
                    },
                    items:
                        Priority.values
                            // .where((p) => p != Task.Priority.unknown)
                            .map(
                              (p) => DropdownMenuItem<Priority>(
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
                  child: DropdownButtonFormField<EstimatedTime>(
                    decoration: const InputDecoration(
                      border: OutlineInputBorder(),
                      contentPadding: EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                    ),
                    value: _estimatedTime,
                    onChanged: (EstimatedTime? value) {
                      if (value != null) {
                        setState(() {
                          _estimatedTime = value;
                        });
                      }
                    },
                    items:
                        EstimatedTime.values
                            // .where((time) => time != Task.EstimatedTime.unknown)
                            .map(
                              (t) => DropdownMenuItem<EstimatedTime>(
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
                  child: DropdownButtonFormField<EstimatedEnergy>(
                    decoration: const InputDecoration(
                      border: OutlineInputBorder(),
                      contentPadding: EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                    ),
                    value: _estimatedEnergy,
                    onChanged: (EstimatedEnergy? value) {
                      if (value != null) {
                        setState(() {
                          _estimatedEnergy = value;
                        });
                      }
                    },
                    items:
                        EstimatedEnergy.values
                            // .where((e) => e != Task.EstimatedEnergy.unknown)
                            .map(
                              (e) => DropdownMenuItem<EstimatedEnergy>(
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
    if (_nameController.text.toPlainText().trim().isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Task name cannot be empty')),
      );
      return;
    }

    // Extract description from document
    final description = _descriptionController.text.toPlainText().trim();
    final name = _nameController.text.toPlainText().trim();

    final taskBox = ObjectBox.taskBox();

    if (widget.task == null) {
      // Create new task
      final newTask = Task.Task(
        uuid: const Uuid().v4(),
        name: name,
        description: description,
        createDate: DateTime.now(),
        modifyDate: DateTime.now(),
        dueDate: _dueDate,
        status: _status,
        priority: _priority,
        estimatedTime: _estimatedTime,
        estimatedEnergy: _estimatedEnergy,
      );

      // Set project if selected
      if (_selectedProject != null) {
        newTask.taskList.target = _selectedProject;
      }

      taskBox.put(newTask);
    } else {
      // Update existing task
      widget.task!.name = name;
      widget.task!.description = description;
      widget.task!.modifyDate = DateTime.now();
      widget.task!.dueDate = _dueDate;
      widget.task!.status = _status;
      widget.task!.priority = _priority;
      widget.task!.estimatedTime = _estimatedTime;
      widget.task!.estimatedEnergy = _estimatedEnergy;

      // Update project association
      widget.task!.taskList.target = _selectedProject;

      taskBox.put(widget.task!);
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
