import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../editor/editor.dart';
import '../../infrastructure/logging.dart';
import '../../objectbox/objectbox.g.dart';
import '../../styling/tempo_stylesheet.dart';
import '../../things/thing_factories.dart';
import '../../things/task/task.entity.dart';
import '../../things/task/task_repository.dart';
import '../../things/thing/interface.dart';
import '../widgets/panel/panel_appbar.dart';
import '../widgets/panel/panel_header.dart';

/// Sliver-based panel for displaying tasks list using SuperEditor
/// This version uses CustomScrollView with SuperEditor as a Sliver
class PanelWrapper<BT extends BaseThing> extends ConsumerStatefulWidget {
  const PanelWrapper({
    super.key,
    required this.header,
    required this.stream,
    required this.emptyWidget,
    this.actions = const [],
    this.routePath,
  });

  final PanelHeaderMixin header;
  final Stream<BT> stream;
  final Widget emptyWidget;
  final List<Widget> actions;
  final String? routePath;

  @override
  ConsumerState<PanelWrapper<BT>> createState() => _PanelWrapperState<BT>();
}

class _PanelWrapperState<BT extends BaseThing>
    extends ConsumerState<PanelWrapper<BT>> {
  late MutableDocument _document;
  late MutableDocumentComposer _composer;
  late HoverManager _hoverManager;
  late LeaderLinkManager _linkManager;
  StreamSubscription<BT>? _subscription;

  bool _isLoading = true;
  bool _initialLoad = false;

  // 🎯 NEW: Key to access the app bar for scroll updates
  late final GlobalKey<PanelAppBarSliverState> _appBarKey;

  // 🎯 NEW: Key to measure the PanelHeader height for scroll calculations
  late final GlobalKey _headerKey;

  // Extract header title dynamically in build method
  String get _headerTitle => widget.header.controller.text.toPlainText();

  @override
  void initState() {
    super.initState();

    // Initialize GlobalKeys to ensure uniqueness per instance
    _appBarKey = GlobalKey<PanelAppBarSliverState>();
    _headerKey = GlobalKey();

    _hoverManager = HoverManager();
    _linkManager = LeaderLinkManager();

    // 🎯 NEW: Create empty document without header since SliverAppBar displays title
    // The header is redundant when we have a sticky app bar showing the same title
    _document = OrderedSetDocument(nodes: []);
    // Initialize composer
    _composer = MutableDocumentComposer();

    try {
      _loadContents();
    } catch (e) {
      tempoPanelLog.severe('Error initializing panel: $e');
    } finally {
      setState(() {
        _isLoading = false;
      });
    }
  }

  @override
  void dispose() {
    _subscription?.cancel();
    _subscription = null;

    super.dispose();
  }

  void _loadContents() {
    // Set up task stream for reactive updates
    final nodeFactory = ref.read(nodeFactoryRegistryProvider);
    _subscription = widget.stream.listen((BT thing) {
      final node = nodeFactory.toNodeFromEntity(thing);
      tempoPanelLog.finest("Adding ${node.id}");
      _document.add(node);

      if (!_initialLoad) {
        _initialLoad = true; // Set immediately to prevent multiple callbacks
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted) {
            setState(() {
              _isLoading = false;
            });
          }
        });
      }
    });
  }

  // 🎯 NEW: Handle scroll notifications and update app bar
  bool _onScrollNotification(ScrollNotification notification) {
    // Don't consume the notification
    if (notification is! ScrollUpdateNotification) {
      return false;
    }

    final scrollPixels = notification.metrics.pixels;
    tempoPanelLog.finest('ScrollNotification: ${scrollPixels}px');

    // 🎯 NEW: Calculate header height dynamically for transition threshold
    double headerHeight = 100.0; // Default fallback

    final headerRenderBox =
        _headerKey.currentContext?.findRenderObject() as RenderBox?;
    if (headerRenderBox != null) {
      headerHeight = headerRenderBox.size.height;
      tempoPanelLog.finest('PanelHeader height: ${headerHeight}px');
    }

    // 🎯 NEW: Pass both scroll position and header height to app bar
    _appBarKey.currentState?.updateScrollPosition(scrollPixels, headerHeight);
    return true;
  }

  @override
  Widget build(BuildContext context) {
    final stylesheet = TempoStylesheet.of(ref);

    tempoPanelLog.finer("Currently loading ${_document.length} nodes");

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
      );
    } catch (e) {
      tempoPanelLog.severe('Error creating SuperEditor: $e');
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
      body: Stack(
        children: [
          NotificationListener<ScrollNotification>(
            onNotification: _onScrollNotification,
            child: CustomScrollView(
              slivers: [
                // 📋 REUSED: Custom app bar with scroll-based animations from PanelAppBar
                PanelAppBar(
                  key: _appBarKey,
                  title: _headerTitle.isNotEmpty ? _headerTitle : 'Untitled',
                  actions: widget.actions,
                ),

                // 📋 REUSED: Page header using the provided PanelHeader widget
                // 🎯 FIXED: Uses TempoStylesheet spacing to match SuperEditor's content area
                SliverToBoxAdapter(
                  child: Center(
                    child: ConstrainedBox(
                      constraints: BoxConstraints(
                        maxWidth: stylesheet.spacing.maxContentWidth,
                      ),
                      child: Padding(
                        padding: EdgeInsets.symmetric(
                          horizontal: stylesheet.spacing.contentHorizontal,
                        ),
                        child: Container(
                          key:
                              _headerKey, // 🎯 NEW: Key to measure height for scroll calculations
                          child: widget.header,
                        ),
                      ),
                    ),
                  ),
                ),

                // SuperEditor as a sliver - this is the key change!
                superEditorSliver,
              ],
            ),
          ),

          if (_isLoading) Center(child: CircularProgressIndicator()),

          if (!_isLoading && _document.isEmpty) widget.emptyWidget,
        ],
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
