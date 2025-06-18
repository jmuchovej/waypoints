import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';

import '../../ui/widgets/task/panel.dart';

@RoutePage(name: 'TasksRoute')
class TasksScreen extends StatelessWidget {
  const TasksScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return const TaskPanel();
  }
}
