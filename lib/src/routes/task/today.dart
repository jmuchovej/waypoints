import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';

import '../../ui/widgets/task/panel.dart';

@RoutePage(name: 'TasksTodayRoute')
class TasksTodayPage extends StatelessWidget {
  const TasksTodayPage({super.key});

  @override
  Widget build(BuildContext context) {
    return const TaskPanel(
      defaultFilter: TaskFilter.today,
      title: 'Today\'s Tasks',
    );
  }
}
