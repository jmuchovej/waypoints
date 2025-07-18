import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../editor/super_editor.dart';
import '../../objectbox/objectbox.g.dart';
import '../../things/task/task.entity.dart';
import '../../things/task/task_repository.dart';
import '../../ui/panel/panel_wrapper.dart';
import '../../ui/widgets/buttons/filter_button.dart';
import '../../ui/widgets/panel/empty_state.dart';
import '../../ui/widgets/panel/panel_header.dart';

class TasksTodayScreen extends ConsumerStatefulWidget {
  const TasksTodayScreen({super.key});

  @override
  ConsumerState<TasksTodayScreen> createState() => _TasksTodayScreenState();
}

class _TasksTodayScreenState extends ConsumerState<TasksTodayScreen> {
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
    final header = PanelHeader.readonly(title: AttributedText("Today's Tasks"));
    return PanelWrapper<TaskEntity>(
      header: header,
      stream: getTasks(),
      emptyWidget: EmptyState(
        title: 'No tasks found',
        subtitle: 'Create a new task by tapping the "+" button.',
        icon: Icons.task_alt_rounded,
        label: "Create Task",
        onPressed: () {},
      ),
      actions: [filterButton()],
    );
  }

  Stream<TaskEntity> getTasks() {
    final repo = ref.read(taskRepositoryProvider);

    final Condition<TaskEntity> filter = repo.active().and(repo.dueToday());

    return repo.streamEntities(filter);
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
