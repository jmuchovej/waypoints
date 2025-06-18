import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:auto_route/auto_route.dart';

import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../router.dart';
import '../../shell/pane_manager.dart';
import '../../../things/task.dart';
import '../../../things/task/task_repository.dart';

/// Panel for displaying tasks list using SuperEditor
/// This is specifically designed for the /routes/task/index page
class TaskPanel extends ConsumerStatefulWidget {
  const TaskPanel({
    super.key,
    this.defaultFilter = TaskFilter.all,
    this.title = 'Tasks',
    this.showFilterBar = true,
  });

  final TaskFilter defaultFilter;
  final String title;
  final bool showFilterBar;

  @override
  ConsumerState<TaskPanel> createState() => _TaskPanelState();
}

class _TaskPanelState extends ConsumerState<TaskPanel> {
  late MutableDocument _document;
  late MutableDocumentComposer _composer;
  late HoverManager _hoverManager;
  late LeaderLinkManager _linkManager;
  final ScrollController _scrollController = ScrollController();
  bool _isLoading = true;
  bool _showAppBarTitle = false;
  String? _documentHeaderNodeId;

  // Task filtering options
  late TaskFilter _currentFilter;
  String _searchQuery = '';

  @override
  void initState() {
    super.initState();

    // Initialize filter from widget
    _currentFilter = widget.defaultFilter;

    // Listen to pane manager changes
    paneManager.addListener(_onPaneStateChanged);

    // Listen to scroll changes
    _scrollController.addListener(_onScroll);

    _initializePanel();
  }

  @override
  void dispose() {
    paneManager.removeListener(_onPaneStateChanged);
    _scrollController.removeListener(_onScroll);
    _scrollController.dispose();
    super.dispose();
  }

  void _initializePanel() async {
    try {
      // Initialize managers
      _hoverManager = HoverManager();
      _linkManager = LeaderLinkManager();

      // Initialize document
      await _initializeDocument();

      setState(() {
        _isLoading = false;
      });
    } catch (e) {
      fluxLog.severe('Error initializing tasks panel: $e');
      setState(() {
        _isLoading = false;
      });
    }
  }

  Future<void> _initializeDocument() async {
    // Create empty document
    final nodeId = Editor.createNodeId();
    _document = MutableDocument.empty(nodeId);
    _document.deleteNode(nodeId);

    // Load tasks and add to document
    await _loadTasks();

    // Initialize composer
    _composer = MutableDocumentComposer();
  }

  Future<void> _loadTasks() async {
    try {
      final taskRepo = ref.read(taskRepositoryProvider);
      var tasks = <TaskEntity>[];

      // Apply filter
      switch (_currentFilter) {
        case TaskFilter.all:
          tasks =
              taskRepo
                  .getAll()
                  .where((t) => !t.archived && !t.deleted)
                  .toList();
          break;
        case TaskFilter.active:
          tasks =
              taskRepo
                  .getAll()
                  .where(
                    (t) => !t.status.isCompleted && !t.archived && !t.deleted,
                  )
                  .toList();
          break;
        case TaskFilter.completed:
          tasks =
              taskRepo
                  .withStatus(TaskStatus.complete)
                  .where((t) => !t.archived && !t.deleted)
                  .toList();
          break;
        case TaskFilter.today:
          tasks =
              taskRepo
                  .dueToday()
                  .where((t) => !t.archived && !t.deleted)
                  .toList();
          break;
        case TaskFilter.overdue:
          tasks =
              taskRepo
                  .overdue()
                  .where((t) => !t.archived && !t.deleted)
                  .toList();
          break;
        case TaskFilter.highPriority:
          tasks =
              taskRepo
                  .withPriority(TaskPriority.H)
                  .where((t) => !t.archived && !t.deleted)
                  .toList();
          break;
      }

      // Apply search filter
      if (_searchQuery.isNotEmpty) {
        tasks =
            tasks
                .where(
                  (task) => task.title.toPlainText().toLowerCase().contains(
                    _searchQuery.toLowerCase(),
                  ),
                )
                .toList();
      }

      // Only show top-level tasks (no subtasks)
      tasks = tasks.where((task) => task.parent.target == null).toList();

      // Sort tasks by urgency (highest first), then by due date
      tasks.sort((a, b) {
        final urgencyComparison = b.urgency.compareTo(a.urgency);
        if (urgencyComparison != 0) return urgencyComparison;

        return a.dueDate.compareTo(b.dueDate);
        // return b.modifyDate.compareTo(a.modifyDate);
      });

      fluxLog.info('Loaded ${tasks.length} tasks with filter: $_currentFilter');

      // Add tasks to document as TaskNodes
      for (final task in tasks) {
        final taskNode = TaskNode.fromTask(task);
        _document.add(taskNode);
      }

      fluxLog.info('Added ${tasks.length} task nodes to document');
    } catch (e) {
      fluxLog.severe('Error loading tasks: $e');
    }
  }

  Future<void> _refreshTasks() async {
    setState(() {
      _isLoading = true;
    });

    // Clear existing document (except header)
    _document.deleteAllButHeader();

    // Reload tasks
    await _loadTasks();

    setState(() {
      _isLoading = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final screenWidth = MediaQuery.of(context).size.width;
    final isMobile = screenWidth <= 768;

    // Add document header for desktop if not already present
    final shouldAddHeader =
        !isMobile &&
        (_document.isEmpty || _document.firstOrNull is! DocumentHeaderNode);

    if (shouldAddHeader) {
      final nodeId = Editor.createNodeId();
      final headerNode = DocumentHeaderNode(id: nodeId, title: widget.title);

      _document.insertNodeAt(0, headerNode);
      _documentHeaderNodeId = nodeId;
    }

    // Create SuperEditor
    Widget superEditor;
    if (_isLoading) {
      superEditor = const Center(child: CircularProgressIndicator());
    } else if (_document.isEmpty ||
        (_document.length == 1 &&
            _document.firstOrNull is DocumentHeaderNode)) {
      superEditor = _buildEmptyState();
    } else {
      try {
        superEditor = createSuperEditor(
          context,
          _document,
          _composer,
          hoverManager: _hoverManager,
          linkManager: _linkManager,
          scrollController: _scrollController,
        );
      } catch (e) {
        fluxLog.severe('Error creating SuperEditor: $e');
        superEditor = Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Icon(Icons.error, size: 48, color: Colors.red),
              const SizedBox(height: 16),
              Text('Error loading tasks: $e'),
              const SizedBox(height: 16),
              ElevatedButton(
                onPressed: _refreshTasks,
                child: const Text('Retry'),
              ),
            ],
          ),
        );
      }
    }

    final content = Container(
      color: Theme.of(context).colorScheme.surfaceContainer,
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 800),
          child: Column(
            children: [
              // Filter bar (on mobile and desktop)
              if (widget.showFilterBar) _buildFilterBar(),
              Expanded(child: superEditor),
            ],
          ),
        ),
      ),
    );

    // Build app bar
    final appBar = AppBar(
      backgroundColor: Theme.of(context).colorScheme.surfaceContainer,
      elevation: 0,
      scrolledUnderElevation: 0,
      title: AnimatedSlide(
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeInOut,
        offset: Offset(0, _showAppBarTitle ? 0 : 1),
        child: AnimatedOpacity(
          duration: const Duration(milliseconds: 200),
          opacity: _showAppBarTitle ? 1.0 : 0.0,
          child: Text(widget.title),
        ),
      ),
      actions: [
        IconButton(
          icon: const Icon(Icons.search),
          onPressed: _showSearchDialog,
          tooltip: 'Search',
        ),
        IconButton(
          icon: const Icon(Icons.refresh),
          onPressed: _refreshTasks,
          tooltip: 'Refresh',
        ),
        IconButton(
          icon: const Icon(Icons.add),
          onPressed: () => _createNewTask(),
          tooltip: 'New Task',
        ),
      ],
    );

    // Return layout based on mobile/desktop
    if (isMobile) {
      return Scaffold(appBar: appBar, body: content);
    }

    // Desktop layout with fixed app bar
    return Scaffold(
      body: Column(
        children: [
          appBar,
          AnimatedContainer(
            duration: const Duration(milliseconds: 200),
            height: 1,
            color:
                _showAppBarTitle
                    ? Theme.of(context).colorScheme.outlineVariant
                    : Theme.of(context).colorScheme.surfaceContainer,
          ),
          Expanded(child: content),
        ],
      ),
    );
  }

  Widget _buildFilterBar() {
    final theme = Theme.of(context);

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children:
              TaskFilter.values.map((filter) {
                final isSelected = _currentFilter == filter;
                return Padding(
                  padding: const EdgeInsets.only(right: 8),
                  child: FilterChip(
                    label: Text(filter.displayName),
                    selected: isSelected,
                    onSelected: (selected) {
                      if (selected) {
                        setState(() {
                          _currentFilter = filter;
                        });
                        _refreshTasks();
                      }
                    },
                    backgroundColor: theme.colorScheme.surfaceContainerHighest,
                    selectedColor: theme.colorScheme.primaryContainer,
                  ),
                );
              }).toList(),
        ),
      ),
    );
  }

  Widget _buildEmptyState() {
    final theme = Theme.of(context);

    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.task_alt,
            size: 80,
            color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.6),
          ),
          const SizedBox(height: 16),
          Text(
            _getEmptyStateTitle(),
            style: theme.textTheme.headlineSmall?.copyWith(
              fontWeight: FontWeight.bold,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            _getEmptyStateSubtitle(),
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 24),
          ElevatedButton.icon(
            icon: const Icon(Icons.add),
            label: const Text('Create Task'),
            onPressed: _createNewTask,
          ),
        ],
      ),
    );
  }

  String _getEmptyStateTitle() {
    switch (_currentFilter) {
      case TaskFilter.all:
        return 'No tasks yet';
      case TaskFilter.active:
        return 'No active tasks';
      case TaskFilter.completed:
        return 'No completed tasks';
      case TaskFilter.today:
        return 'No tasks due today';
      case TaskFilter.overdue:
        return 'No overdue tasks';
      case TaskFilter.highPriority:
        return 'No high priority tasks';
    }
  }

  String _getEmptyStateSubtitle() {
    if (_searchQuery.isNotEmpty) {
      return 'No tasks match your search';
    }

    switch (_currentFilter) {
      case TaskFilter.all:
        return 'Create your first task to get started';
      case TaskFilter.active:
        return 'All tasks are completed or you need to create new ones';
      case TaskFilter.completed:
        return 'Complete some tasks to see them here';
      case TaskFilter.today:
        return 'No tasks are due today';
      case TaskFilter.overdue:
        return 'Great! You have no overdue tasks';
      case TaskFilter.highPriority:
        return 'No high priority tasks at the moment';
    }
  }

  void _createNewTask() async {
    await context.router.push(CreateTaskRoute());
    // Refresh tasks after creation
    await _refreshTasks();
  }

  void _showSearchDialog() {
    showDialog(
      context: context,
      builder:
          (context) => AlertDialog(
            title: const Text('Search Tasks'),
            content: TextField(
              onChanged: (value) => _searchQuery = value,
              decoration: const InputDecoration(
                hintText: 'Enter search term...',
                prefixIcon: Icon(Icons.search),
              ),
              autofocus: true,
            ),
            actions: [
              TextButton(
                onPressed: () {
                  Navigator.pop(context);
                  setState(() {
                    _searchQuery = '';
                  });
                  _refreshTasks();
                },
                child: const Text('Clear'),
              ),
              TextButton(
                onPressed: () {
                  Navigator.pop(context);
                  _refreshTasks();
                },
                child: const Text('Search'),
              ),
            ],
          ),
    );
  }

  void _onPaneStateChanged() {
    setState(() {});
  }

  void _onScroll() {
    if (_documentHeaderNodeId == null) return;

    final shouldShowTitle = _scrollController.offset > 100;

    if (shouldShowTitle != _showAppBarTitle) {
      setState(() {
        _showAppBarTitle = shouldShowTitle;
      });
    }
  }
}

/// Task filter options for the tasks panel
enum TaskFilter { all, active, completed, today, overdue, highPriority }

extension TaskFilterExtension on TaskFilter {
  String get displayName {
    switch (this) {
      case TaskFilter.all:
        return 'All';
      case TaskFilter.active:
        return 'Active';
      case TaskFilter.completed:
        return 'Completed';
      case TaskFilter.today:
        return 'Due Today';
      case TaskFilter.overdue:
        return 'Overdue';
      case TaskFilter.highPriority:
        return 'High Priority';
    }
  }
}
