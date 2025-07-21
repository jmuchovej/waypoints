import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../editor/super_editor.dart';
import '../../infrastructure/logging.dart';
import '../../ui/widgets/panel/panel_header.dart';
import '../../things/task/task.entity.dart';
import '../../objectbox/repositories/task_repository.dart';
import '../../ui/panel/panel_wrapper.dart';
import '../../ui/widgets/buttons/filter_button.dart';
import '../../ui/widgets/panel/empty_state.dart';

class TasksScreen extends ConsumerStatefulWidget {
  const TasksScreen({super.key});

  @override
  ConsumerState<TasksScreen> createState() => _TasksScreenState();
}

class _TasksScreenState extends ConsumerState<TasksScreen> with RouteAware {
  StreamController<TaskEntity>? _streamController;
  StreamSubscription<TaskEntity>? _sourceSubscription;
  bool _isPaused = false;

  @override
  void initState() {
    super.initState();
    _initializeStream();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    // Listen to route changes
    final router = GoRouter.of(context);
    router.routerDelegate.addListener(_onRouteChanged);
  }

  void _onRouteChanged() {
    final router = GoRouter.of(context);
    final location = router.routerDelegate.currentConfiguration.fullPath;
    final isActive = location == '/tasks';

    if (isActive && _isPaused) {
      _resumeStream();
    } else if (!isActive && !_isPaused) {
      _pauseStream();
    }
  }

  void _initializeStream() {
    _streamController = StreamController<TaskEntity>.broadcast();

    // Start paused if not on /tasks
    final router = GoRouter.of(context);
    final isActive = router.routerDelegate.currentConfiguration.fullPath == '/tasks';
    _isPaused = !isActive;

    if (!_isPaused) {
      _startSourceSubscription();
    }
  }

  void _startSourceSubscription() {
    _sourceSubscription?.cancel();
    _sourceSubscription = _getTasksSource().listen(
      (task) {
        if (!_isPaused) {
          _streamController?.add(task);
        }
      },
      onError: (error) => _streamController?.addError(error),
      onDone: () => _streamController?.close(),
    );
  }

  void _pauseStream() {
    _isPaused = true;
    _sourceSubscription?.pause();
    tempoTaskRouteLog.fine('Paused task stream');
  }

  void _resumeStream() {
    _isPaused = false;
    if (_sourceSubscription?.isPaused ?? false) {
      _sourceSubscription?.resume();
    } else {
      // Restart if needed
      _startSourceSubscription();
    }
    tempoTaskRouteLog.fine('Resumed task stream');
  }

  @override
  void dispose() {
    final router = GoRouter.of(context);
    router.routerDelegate.removeListener(_onRouteChanged);
    _sourceSubscription?.cancel();
    _streamController?.close();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final emptyWidget = EmptyState(
      title: 'No tasks found',
      subtitle: 'Create a new task by tapping the "+" button.',
      icon: Icons.task_alt_rounded,
      label: "Create Task",
      onPressed: () {},
    );

    final header = PanelHeader.readonly(title: AttributedText('Tasks'));

    return PanelWrapper<TaskEntity>(
      header: header,
      stream: _streamController?.stream ?? Stream.empty(),
      emptyWidget: emptyWidget,
      routePath: '/tasks',
      actions: [filterButton()],
    );
  }

  Stream<TaskEntity> _getTasksSource() {
    final repo = ref.read(taskRepositoryProvider);

    final before = ProcessInfo.currentRss;
    final tasks = repo.getAll();

    final after = ProcessInfo.currentRss;
    final perTask = (after - before) / tasks.length;
    tempoTaskRouteLog.finer("Average memory per task: ${perTask} bytes");

    // Stream top-level active tasks and flatten to individual entities
    return repo
        .streamTopLevel()
        .map((tasks) {
          tempoTaskRouteLog.info('received ${tasks.length} top-level tasks');
          return tasks;
        })
        .asyncExpand((tasks) async* {
          for (final task in tasks) {
            tempoTaskRouteLog.info(
              'yielding task: ${task.uuid} - ${task.title.toPlainText()}',
            );
            yield task;
          }
        });
  }

  FilterButton filterButton() {
    final repo = ref.read(taskRepositoryProvider);

    // final options =
    //     TaskFilter.values
    //         .map(
    //           (filter) => FilterOption(
    //             icon: filter.icon,
    //             title: filter.displayName,
    //             onPressed: () {
    //               filter.filterTasks(repo);
    //               setState(() {});
    //             },
    //           ),
    //         )
    //         .toList();

    return FilterButton(
      icon: Icons.filter_list_rounded,
      label: 'Filter',
      options: const [], // options,
    );
  }
}
