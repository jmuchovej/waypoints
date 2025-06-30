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
import '../widgets/panel/panel_appbar_sliver.dart';
import '../widgets/panel/panel_header.dart';

/// Sliver-based panel for displaying tasks list using SuperEditor
/// This version uses CustomScrollView with SuperEditor as a Sliver
class PreconfiguredPanelSliver<T extends Thing> extends ConsumerStatefulWidget {
  const PreconfiguredPanelSliver({
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
  ConsumerState<PreconfiguredPanelSliver<T>> createState() =>
      _PreconfiguredPanelSliverState<T>();
}

class _PreconfiguredPanelSliverState<T extends Thing>
    extends ConsumerState<PreconfiguredPanelSliver<T>> {
  late MutableDocument _document;
  late MutableDocumentComposer _composer;
  late HoverManager _hoverManager;
  late LeaderLinkManager _linkManager;
  StreamSubscription<T>? _subscription;

  bool _isLoading = true;
  bool _initialLoad = false;
  bool _isDisposed = false;

  // 🎯 NEW: Key to access the app bar for scroll updates
  final GlobalKey<PanelAppBarSliverState> _appBarKey = GlobalKey<PanelAppBarSliverState>();

  // 📋 REUSED: Controller and focus node for PanelHeader
  late AttributedTextEditingController _titleController;
  late FocusNode _titleFocusNode;

  // 🎯 NEW: Key to measure the PanelHeader height for scroll calculations
  final GlobalKey _headerKey = GlobalKey();

  @override
  void initState() {
    super.initState();

    _hoverManager = HoverManager();
    _linkManager = LeaderLinkManager();

    // 📋 REUSED: Initialize title controller and focus node for PanelHeader
    _titleController = AttributedTextEditingController(
      text: widget.header.text,
    );
    _titleFocusNode = FocusNode();

    // 🎯 NEW: Create empty document without header since SliverAppBar displays title
    // The header is redundant when we have a sticky app bar showing the same title
    _document = OrderedSetDocument(nodes: []);
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
    _subscription?.cancel();
    _subscription = null;
    
    // 📋 REUSED: Clean up title controller and focus node
    _titleController.dispose();
    _titleFocusNode.dispose();
    
    super.dispose();
  }

  void _loadContents() {
    // Set up task stream for reactive updates
    final nodeFactory = ref.read(nodeFactoryRegistryProvider);
    _subscription = widget.stream.listen((T thing) {
      if (_isDisposed) return;

      final node = nodeFactory.toNodeFromEntity(thing);
      fluxLog.finest("Adding ${node.id}");
      _document.add(node);

      if (!_initialLoad) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted) {
            setState(() {
              _initialLoad = true;
              _isLoading = false;
            });
          }
        });
      }
    });
  }

  // 🎯 NEW: Handle scroll notifications and update app bar
  bool _onScrollNotification(ScrollNotification notification) {
    if (notification is ScrollUpdateNotification) {
      final scrollPixels = notification.metrics.pixels;
      fluxLog.finest('ScrollNotification: ${scrollPixels}px');
      
      // 🎯 NEW: Calculate header height dynamically for transition threshold
      double headerHeight = 100.0; // Default fallback
      
      final headerRenderBox = _headerKey.currentContext?.findRenderObject() as RenderBox?;
      if (headerRenderBox != null) {
        headerHeight = headerRenderBox.size.height;
        fluxLog.finest('PanelHeader height: ${headerHeight}px');
      }
      
      // 🎯 NEW: Pass both scroll position and header height to app bar
      _appBarKey.currentState?.updateScrollPosition(scrollPixels, headerHeight);
    }
    return false; // Don't consume the notification
  }

  @override
  Widget build(BuildContext context) {
    if (_isLoading || !_initialLoad) {
      return const Scaffold(
        body: Center(child: CircularProgressIndicator()),
      );
    }

    // 🎯 UPDATED: Check for empty document (no header node now)
    if (_initialLoad && _document.length == 0) {
      return Scaffold(body: widget.emptyWidget);
    }

    final screenWidth = MediaQuery.of(context).size.width;
    final isMobile = screenWidth <= 768;

    fluxLog.finest("Currently loading ${_document.length} nodes");

    // Create SuperEditor as a Sliver using the centralized factory
    Widget superEditorSliver;
    try {
      superEditorSliver = createSuperEditor(
        context,
        _document,
        _composer,
        hoverManager: _hoverManager,
        linkManager: _linkManager,
        taskRepository: ref.read(taskRepositoryProvider),
        ref: ref,
        autofocus: false,
        renderMode: SuperEditorRenderMode.sliver, // 🎯 This is the key change!
      );
    } catch (e) {
      fluxLog.severe('Error creating SuperEditor: $e');
      superEditorSliver = SliverToBoxAdapter(
        child: Center(
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
        ),
      );
    }

    // 🎯 NEW: Use CustomScrollView with our custom PanelAppBarSliver
    // 🎯 FIXED: Wrap with NotificationListener to catch scroll events
    return Scaffold(
      backgroundColor: Theme.of(context).colorScheme.surfaceContainer,
      body: NotificationListener<ScrollNotification>(
        onNotification: _onScrollNotification,
        child: CustomScrollView(
          slivers: [
            // 📋 REUSED: Custom app bar with scroll-based animations from PanelAppBar
            PanelAppBarSliver(
              key: _appBarKey,
              title: widget.header.text.toPlainText(),
              actions: widget.actions,
            ),

            // 📋 REUSED: Page header using the existing PanelHeader widget
            SliverToBoxAdapter(
              child: PanelHeader(
                key: _headerKey, // 🎯 NEW: Key to measure height for scroll calculations
                titleController: _titleController,
                titleFocusNode: _titleFocusNode,
                hintText: AttributedText('Page Title'),
                children: widget.actions.isNotEmpty 
                  ? [
                      // Add action buttons below the title if any
                      Padding(
                        padding: const EdgeInsets.only(top: 8.0),
                        child: Row(
                          children: widget.actions,
                        ),
                      ),
                    ]
                  : [],
              ),
            ),

            // SuperEditor as a sliver - this is the key change!
            superEditorSliver,
          ],
        ),
      ),
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
