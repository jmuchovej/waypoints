import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../editor/editor.dart';
import '../../infrastructure/logging.dart';
import '../../objectbox/objectbox.g.dart';
import '../../things/node_factory.dart';
import '../../things/task/task_entity.dart';
import '../../things/task/task_repository.dart';
import '../../things/thing/interface.dart';
import '../widgets/buttons/filter_button.dart';
import '../widgets/panel/panel_appbar.dart';

/// Panel for displaying tasks list using SuperEditor
/// This is specifically designed for the /routes/task/index page
class PreconfiguredPanel<T extends Thing> extends ConsumerStatefulWidget {
  const PreconfiguredPanel({
    super.key,
    required this.header,
    required this.stream,
    required this.emptyWidget,
    this.actions = const [],
  });

  final EditableTextNode header;
  final Stream<T> stream;
  final Widget emptyWidget;
  final List<Widget> actions;

  @override
  ConsumerState<PreconfiguredPanel<T>> createState() =>
      _PreconfiguredPanelState<T>();
}

class _PreconfiguredPanelState<T extends Thing>
    extends ConsumerState<PreconfiguredPanel<T>> {
  late MutableDocument _document;
  late MutableDocumentComposer _composer;
  late HoverManager _hoverManager;
  late LeaderLinkManager _linkManager;
  ScrollController? _scrollController;
  StreamSubscription<T>? _subscription;

  bool _isLoading = true;
  bool _initialLoad = false;
  bool _isDisposed = false;

  @override
  void initState() {
    super.initState();

    // Initialize scroll controller with debugging
    _scrollController = ScrollController();

    // Add scroll listener for debugging
    _scrollController?.addListener(() {
      if (_scrollController?.hasClients ?? false) {
        fluxLog.finest(
          '📜 Scroll position: ${_scrollController!.offset.toStringAsFixed(1)} / ${_scrollController!.position.maxScrollExtent.toStringAsFixed(1)}',
        );
      }
    });

    _hoverManager = HoverManager();
    _linkManager = LeaderLinkManager();

    // Create empty document
    _document = OrderedSetDocument(nodes: [widget.header]);
    // Initialize composer
    _composer = MutableDocumentComposer();

    try {
      _loadContents();
    } catch (e) {
      fluxLog.severe('Error initializing panel: $e');
    } finally {
      setState(() {
        _isLoading = false;
      });
    }
  }

  @override
  void dispose() {
    _isDisposed = true;
    _scrollController?.dispose();
    _scrollController = null;
    _subscription?.cancel();
    _subscription = null;
    super.dispose();
  }

  void _loadContents() {
    // _subscription.cancel();

    // Set up task stream for reactive updates
    final nodeFactory = ref.read(nodeFactoryRegistryProvider);
    _subscription = widget.stream.listen((T thing) {
      final node = nodeFactory.toNodeFromEntity(thing);
      fluxLog.finest("Adding ${node.id}");
      _document.add(node);

      if (!_initialLoad) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          setState(() {
            _initialLoad = true;
            _isLoading = false;
          });
        });
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    if (_isLoading || !_initialLoad) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_initialLoad && _document.length < 2) {
      return widget.emptyWidget;
    }

    final screenWidth = MediaQuery.of(context).size.width;
    final isMobile = screenWidth <= 768;

    fluxLog.finest("Currently loading ${_document.length} nodes");

    // Create SuperEditor
    Widget superEditor;
    try {
      superEditor = createSuperEditor(
        context,
        _document,
        _composer,
        hoverManager: _hoverManager,
        linkManager: _linkManager,
        taskRepository: ref.read(taskRepositoryProvider),
        ref: ref,
        scrollController: _scrollController!,
        autofocus: false, // Explicitly disable autofocus in task list
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
              onPressed: _loadContents,
              child: const Text('Retry'),
            ),
          ],
        ),
      );
    }

    final content = Container(
      color: Theme.of(context).colorScheme.surfaceContainer,
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: double.infinity),
          child: Column(
            children: [
              // Filter bar (on mobile and desktop)
              Expanded(child: superEditor),
            ],
          ),
        ),
      ),
    );

    // Desktop: Use PanelAppBar with scroll-based animations
    final headerSize = PanelAppBar.computePreferredSize(context);
    final header = PanelAppBar(
      title: widget.header.text.toPlainText(),
      size: headerSize,
      scrollController: _scrollController,
      actions: widget.actions,
    );

    if (isMobile) {
      return Scaffold(appBar: header, body: content);
    }

    return Scaffold(
      appBar: header,
      body: Column(children: [Expanded(child: content)]),
    );
  }
}

/// Task filter options for the tasks panel
enum TaskFilter {
  all(
    Icons.checklist_rounded,
    'No tasks yet',
    'Create your first task to get started',
  ),
  incomplete(
    Icons.radio_button_unchecked_rounded,
    'No active tasks',
    'All tasks are completed or you need to create new ones',
  ),
  completed(
    Icons.check_circle_rounded,
    'No completed tasks',
    'Complete some tasks to see them here',
  ),
  today(Icons.today_rounded, 'No tasks are due today', null),
  overdue(
    Icons.av_timer_rounded,
    'No overdue tasks',
    'Great! No tasks are overdue',
  ),
  highPriority(Icons.flag_rounded, 'No high priority tasks', null);

  const TaskFilter(this.icon, this.title, this.subtitle);

  final IconData icon;
  final String title;
  final String? subtitle;
}

extension TaskFilterExtension on TaskFilter {
  String get displayName {
    switch (this) {
      case TaskFilter.all:
        return 'All';
      case TaskFilter.incomplete:
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

  Condition<TaskEntity> filterTasks(TaskRepository repo) {
    Condition<TaskEntity> filter = repo.active();
    switch (this) {
      case TaskFilter.all:
        break;
      case TaskFilter.incomplete:
        filter = filter.and(repo.incomplete());
        break;
      case TaskFilter.completed:
        filter = filter.and(TaskStatusGroups.isCompleted);
        break;
      case TaskFilter.today:
        filter = filter.and(repo.dueToday());
        break;
      case TaskFilter.overdue:
        filter = filter.and(repo.overdue());
        break;
      case TaskFilter.highPriority:
        filter = filter.and(repo.withPriority(TaskPriority.H));
        break;
    }

    return filter;
  }
}
