import 'package:flutter/material.dart';
import 'package:flux/ui/widgets/task.dart';
import 'package:uuid/uuid.dart';
import 'package:flux/models/task.dart';
import '../../models/task_list.dart';
import '../../objectbox.dart';
import '../../utils/date_formatter.dart';
import './task_editor.dart';

class ProjectsScreen extends StatefulWidget {
  final TaskList? initialTaskList;

  ProjectsScreen({super.key, this.initialTaskList});

  @override
  _ProjectsScreenState createState() => _ProjectsScreenState();
}

class _ProjectsScreenState extends State<ProjectsScreen> {
  List<TaskList> _taskLists = [];
  List<Task> _tasks = [];
  TaskList? _selectedTaskList;
  bool _isLoading = true;

  @override
  void initState() {
    super.initState();
    _selectedTaskList = widget.initialTaskList;
    _loadData();
  }

  void _loadData() async {
    setState(() {
      _isLoading = true;
    });

    // Load all task lists
    _taskLists = ObjectBox.taskListBox().getAll();

    if (_selectedTaskList != null) {
      // Make sure we have the latest version from the database
      final refreshedList = ObjectBox.taskListBox().get(_selectedTaskList!.id);
      if (refreshedList != null) {
        _selectedTaskList = refreshedList;
        // Load tasks for selected project
        _tasks = _selectedTaskList!.tasks.toList();
      } else {
        _tasks = [];
      }
    } else if (_taskLists.isNotEmpty) {
      // Select the first task list by default
      _selectedTaskList = _taskLists.first;
      _tasks = _selectedTaskList!.tasks.toList();
    } else {
      _tasks = [];
    }

    setState(() {
      _isLoading = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text('Projects'),
        actions: [
          IconButton(icon: Icon(Icons.add), onPressed: _createNewTaskList),
        ],
      ),
      body:
          _isLoading
              ? Center(child: CircularProgressIndicator())
              : _buildMainContent(),
    );
  }

  Widget _buildMainContent() {
    if (_taskLists.isEmpty) {
      return _buildEmptyState();
    }

    // Use LayoutBuilder to adapt to different screen sizes
    return LayoutBuilder(
      builder: (context, constraints) {
        // For larger screens, use a side-by-side layout
        if (constraints.maxWidth > 600) {
          return Row(
            children: [
              // Left sidebar for task lists
              Container(
                width: 250,
                decoration: BoxDecoration(
                  border: Border(
                    right: BorderSide(color: Colors.grey.shade300),
                  ),
                ),
                child: _buildTaskListSidebar(),
              ),

              // Right side for tasks in selected list
              Expanded(
                child:
                    _selectedTaskList == null
                        ? Center(child: Text('Select a project'))
                        : _buildTaskListContent(),
              ),
            ],
          );
        } else {
          // For smaller screens, use a more compact layout
          return Column(
            children: [
              // Top bar with dropdown for task lists
              Container(
                padding: EdgeInsets.all(16),
                decoration: BoxDecoration(
                  border: Border(
                    bottom: BorderSide(color: Colors.grey.shade300),
                  ),
                ),
                child: _buildTaskListDropdown(),
              ),

              // Tasks in selected list
              Expanded(
                child:
                    _selectedTaskList == null
                        ? Center(child: Text('Select a project'))
                        : _buildTaskListContent(),
              ),
            ],
          );
        }
      },
    );
  }

  Widget _buildEmptyState() {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(Icons.folder_open, size: 80, color: Colors.grey[400]),
          SizedBox(height: 16),
          Text(
            'No projects yet',
            style: TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
          ),
          SizedBox(height: 8),
          Text(
            'Create a project to organize your tasks',
            style: TextStyle(color: Colors.grey[600]),
          ),
          SizedBox(height: 24),
          ElevatedButton.icon(
            icon: Icon(Icons.add),
            label: Text('Create Project'),
            onPressed: _createNewTaskList,
          ),
        ],
      ),
    );
  }

  Widget _buildTaskListSidebar() {
    return ListView.builder(
      itemCount: _taskLists.length,
      itemBuilder: (context, index) {
        final taskList = _taskLists[index];
        return ListTile(
          title: Text(taskList.name),
          subtitle: Text(
            '${taskList.tasks.length} tasks',
            style: TextStyle(fontSize: 12),
          ),
          selected: _selectedTaskList?.id == taskList.id,
          onTap: () {
            setState(() {
              _selectedTaskList = taskList;
              _tasks = taskList.tasks.toList();
            });
          },
          trailing: PopupMenuButton(
            icon: Icon(Icons.more_vert),
            itemBuilder:
                (context) => [
                  PopupMenuItem(value: 'edit', child: Text('Edit')),
                  PopupMenuItem(value: 'delete', child: Text('Delete')),
                ],
            onSelected: (value) {
              if (value == 'edit') {
                _editTaskList(taskList);
              } else if (value == 'delete') {
                _deleteTaskList(taskList);
              }
            },
          ),
        );
      },
    );
  }

  Widget _buildTaskListDropdown() {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Expanded(
          child: DropdownButton<TaskList>(
            value: _selectedTaskList,
            isExpanded: true,
            hint: Text('Select a project'),
            onChanged: (TaskList? value) {
              if (value != null) {
                setState(() {
                  _selectedTaskList = value;
                  _tasks = value.tasks.toList();
                });
              }
            },
            items:
                _taskLists.map((TaskList taskList) {
                  return DropdownMenuItem<TaskList>(
                    value: taskList,
                    child: Text(taskList.name),
                  );
                }).toList(),
          ),
        ),
        IconButton(
          icon: Icon(Icons.edit),
          onPressed:
              _selectedTaskList != null
                  ? () => _editTaskList(_selectedTaskList!)
                  : null,
        ),
      ],
    );
  }

  Widget _buildTaskListContent() {
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(16.0),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    _selectedTaskList!.name,
                    style: TextStyle(fontSize: 24, fontWeight: FontWeight.bold),
                  ),
                  if (_selectedTaskList!.description.isNotEmpty)
                    Text(
                      _selectedTaskList!.description,
                      style: TextStyle(color: Colors.grey[600]),
                    ),
                ],
              ),
              ElevatedButton.icon(
                icon: Icon(Icons.add),
                label: Text('Add Task'),
                onPressed: () {
                  _addTaskToProject();
                },
              ),
            ],
          ),
        ),
        Divider(),
        Expanded(
          child:
              _tasks.isEmpty
                  ? Center(
                    child: Text(
                      'No tasks in this project',
                      style: TextStyle(color: Colors.grey[600]),
                    ),
                  )
                  : ListView.builder(
                    itemCount: _tasks.length,
                    itemBuilder: (context, index) {
                      final task = _tasks[index];
                      return TaskWidget(
                        task: task,
                        showDueDate: true,
                        onTap: () {
                          Navigator.push(
                            context,
                            MaterialPageRoute(
                              builder:
                                  (context) => TaskEditorScreen(task: task),
                            ),
                          ).then((_) => _loadData());
                        },
                      );
                    },
                  ),
        ),
      ],
    );
  }

  void _createNewTaskList() {
    final nameController = TextEditingController();
    final descriptionController = TextEditingController();

    showDialog(
      context: context,
      builder:
          (context) => AlertDialog(
            title: Text('New Project'),
            content: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                TextField(
                  controller: nameController,
                  decoration: InputDecoration(
                    labelText: 'Project Name',
                    border: OutlineInputBorder(),
                  ),
                  autofocus: true,
                ),
                SizedBox(height: 16),
                TextField(
                  controller: descriptionController,
                  decoration: InputDecoration(
                    labelText: 'Description',
                    border: OutlineInputBorder(),
                  ),
                  maxLines: 3,
                ),
              ],
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context),
                child: Text('Cancel'),
              ),
              ElevatedButton(
                onPressed: () {
                  if (nameController.text.trim().isNotEmpty) {
                    final newTaskList = TaskList(
                      uuid: const Uuid().v4(),
                      name: nameController.text.trim(),
                      description: descriptionController.text.trim(),
                    );

                    ObjectBox.taskListBox().put(newTaskList);
                    Navigator.pop(context);

                    _loadData();
                    setState(() {
                      _selectedTaskList = newTaskList;
                    });
                  }
                },
                child: Text('Create'),
              ),
            ],
          ),
    );
  }

  void _editTaskList(TaskList taskList) {
    final nameController = TextEditingController(text: taskList.name);
    final descriptionController = TextEditingController(
      text: taskList.description,
    );

    showDialog(
      context: context,
      builder:
          (context) => AlertDialog(
            title: Text('Edit Project'),
            content: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                TextField(
                  controller: nameController,
                  decoration: InputDecoration(
                    labelText: 'Project Name',
                    border: OutlineInputBorder(),
                  ),
                  autofocus: true,
                ),
                SizedBox(height: 16),
                TextField(
                  controller: descriptionController,
                  decoration: InputDecoration(
                    labelText: 'Description',
                    border: OutlineInputBorder(),
                  ),
                  maxLines: 3,
                ),
              ],
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context),
                child: Text('Cancel'),
              ),
              ElevatedButton(
                onPressed: () {
                  if (nameController.text.trim().isNotEmpty) {
                    taskList.name = nameController.text.trim();
                    taskList.description = descriptionController.text.trim();

                    ObjectBox.taskListBox().put(taskList);
                    Navigator.pop(context);

                    _loadData();
                  }
                },
                child: Text('Save'),
              ),
            ],
          ),
    );
  }

  void _deleteTaskList(TaskList taskList) {
    showDialog(
      context: context,
      builder:
          (context) => AlertDialog(
            title: Text('Delete Project'),
            content: Text(
              'Are you sure you want to delete "${taskList.name}"? '
              'This will not delete the tasks, but they will no longer be associated with this project.',
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context),
                child: Text('Cancel'),
              ),
              TextButton(
                style: TextButton.styleFrom(foregroundColor: Colors.red),
                onPressed: () {
                  // Remove project association from tasks
                  for (final task in taskList.tasks) {
                    task.taskList.target = null;
                    ObjectBox.taskBox().put(task);
                  }

                  // Delete the task list
                  ObjectBox.taskListBox().remove(taskList.id);
                  Navigator.pop(context);

                  // Select another task list if available
                  _loadData();
                },
                child: Text('Delete'),
              ),
            ],
          ),
    );
  }

  void _addTaskToProject() {
    if (_selectedTaskList == null) return;

    Navigator.push(
      context,
      MaterialPageRoute(
        builder:
            (context) => TaskEditorScreen(projectId: _selectedTaskList!.id),
      ),
    ).then((_) => _loadData());
  }
}
