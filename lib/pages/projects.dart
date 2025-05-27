import 'package:flutter/material.dart';
import 'package:tempo/app.dart';
import 'package:tempo/src/database/objectbox/models/task.dart';
import 'package:tempo/src/database/objectbox/models/project.dart';
import 'package:tempo/pages/task_editor.dart';
import 'package:tempo/ui/widgets/task.dart';
import 'package:uuid/uuid.dart';

class ProjectsScreen extends StatefulWidget {
  final ProjectEntity? initialProject;

  const ProjectsScreen({super.key, this.initialProject});

  @override
  State<ProjectsScreen> createState() => _ProjectsScreenState();
}

class _ProjectsScreenState extends State<ProjectsScreen> {
  List<ProjectEntity> _projects = [];
  List<TaskEntity> _tasks = [];
  ProjectEntity? _selectedProject;
  bool _isLoading = true;

  @override
  void initState() {
    super.initState();
    _selectedProject = widget.initialProject;
    _loadData();
  }

  void _loadData() async {
    setState(() {
      _isLoading = true;
    });

    // Load all task lists
    _projects = obx.getProjects();

    if (_selectedProject != null) {
      // Make sure we have the latest version from the database
      final refreshedList = obx.getProjectByID(_selectedProject!.id);
      if (refreshedList != null) {
        _selectedProject = refreshedList;
        // Load tasks for selected project
        _tasks = _selectedProject!.tasks.toList();
      } else {
        _tasks = [];
      }
    } else if (_projects.isNotEmpty) {
      // Select the first task list by default
      _selectedProject = _projects.first;
      _tasks = _selectedProject!.tasks.toList();
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
          IconButton(icon: Icon(Icons.add), onPressed: _createNewProject),
        ],
      ),
      body:
          _isLoading
              ? Center(child: CircularProgressIndicator())
              : _buildMainContent(),
    );
  }

  Widget _buildMainContent() {
    if (_projects.isEmpty) {
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
                child: _buildProjectSidebar(),
              ),

              // Right side for tasks in selected list
              Expanded(
                child:
                    _selectedProject == null
                        ? Center(child: Text('Select a project'))
                        : _buildProjectContent(),
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
                child: _buildProjectDropdown(),
              ),

              // Tasks in selected list
              Expanded(
                child:
                    _selectedProject == null
                        ? Center(child: Text('Select a project'))
                        : _buildProjectContent(),
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
            onPressed: _createNewProject,
          ),
        ],
      ),
    );
  }

  Widget _buildProjectSidebar() {
    return ListView.builder(
      itemCount: _projects.length,
      itemBuilder: (context, index) {
        final project = _projects[index];
        return ListTile(
          title: Text(project.title),
          subtitle: Text(
            '${project.tasks.length} tasks',
            style: TextStyle(fontSize: 12),
          ),
          selected: _selectedProject?.id == project.id,
          onTap: () {
            setState(() {
              _selectedProject = project;
              _tasks = project.tasks.toList();
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
                _editProject(project);
              } else if (value == 'delete') {
                _deleteProject(project);
              }
            },
          ),
        );
      },
    );
  }

  Widget _buildProjectDropdown() {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Expanded(
          child: DropdownButton<ProjectEntity>(
            value: _selectedProject,
            isExpanded: true,
            hint: Text('Select a project'),
            onChanged: (ProjectEntity? value) {
              if (value != null) {
                setState(() {
                  _selectedProject = value;
                  _tasks = value.tasks.toList();
                });
              }
            },
            items:
                _projects.map((ProjectEntity project) {
                  return DropdownMenuItem<ProjectEntity>(
                    value: project,
                    child: Text(project.title),
                  );
                }).toList(),
          ),
        ),
        IconButton(
          icon: Icon(Icons.edit),
          onPressed:
              _selectedProject != null
                  ? () => _editProject(_selectedProject!)
                  : null,
        ),
      ],
    );
  }

  Widget _buildProjectContent() {
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
                    _selectedProject!.title,
                    style: TextStyle(fontSize: 24, fontWeight: FontWeight.bold),
                  ),
                  if (_selectedProject!.description.isNotEmpty)
                    Text(
                      _selectedProject!.description,
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

  void _createNewProject() {
    final titleController = TextEditingController();
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
                  controller: titleController,
                  decoration: InputDecoration(
                    labelText: 'Project title',
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
                  if (titleController.text.trim().isNotEmpty) {
                    final newProject = ProjectEntity(
                      uuid: const Uuid().v4(),
                      title: titleController.text.trim(),
                      description: descriptionController.text.trim(),
                    );

                    obx.putProject(newProject);
                    Navigator.pop(context);

                    _loadData();
                    setState(() {
                      _selectedProject = newProject;
                    });
                  }
                },
                child: Text('Create'),
              ),
            ],
          ),
    );
  }

  void _editProject(ProjectEntity project) {
    final titleController = TextEditingController(text: project.title);
    final descriptionController = TextEditingController(
      text: project.description,
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
                  controller: titleController,
                  decoration: InputDecoration(
                    labelText: 'Project title',
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
                  if (titleController.text.trim().isNotEmpty) {
                    project.title = titleController.text.trim();
                    project.description = descriptionController.text.trim();

                    obx.putProject(project);
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

  void _deleteProject(ProjectEntity project) {
    showDialog(
      context: context,
      builder:
          (context) => AlertDialog(
            title: Text('Delete Project'),
            content: Text(
              'Are you sure you want to delete "${project.title}"? '
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
                  for (final task in project.tasks) {
                    task.project.target = null;
                    obx.putTask(task);
                  }

                  // Delete the task list
                  obx.deleteProject(project);
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
    if (_selectedProject == null) return;

    Navigator.push(
      context,
      MaterialPageRoute(
        builder: (context) => TaskEditorScreen(projectId: _selectedProject!.id),
      ),
    ).then((_) => _loadData());
  }
}
