import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../editor/super_editor.dart';
import '../../infrastructure/logging.dart';
import '../../ui/widgets/panel/panel_header.dart';
import '../../things/task/task.entity.dart';
import '../../things/task/task_repository.dart';
import '../../ui/panel/panel_wrapper.dart';
import '../../ui/widgets/buttons/filter_button.dart';
import '../../ui/widgets/panel/empty_state.dart';

class TasksScreen extends ConsumerStatefulWidget {
  const TasksScreen({super.key});

  @override
  ConsumerState<TasksScreen> createState() => _TasksScreenState();
}

class _TasksScreenState extends ConsumerState<TasksScreen> {
  @override
  void initState() {
    super.initState();
  }

  @override
  void dispose() {
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
      stream: getTasks(),
      emptyWidget: emptyWidget,
      routePath: '/tasks',
      actions: [filterButton()],
    );
  }

  Stream<TaskEntity> getTasks() {
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
