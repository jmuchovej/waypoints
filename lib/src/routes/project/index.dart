import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';

import '../../ui/widgets/project/panel.dart';

@RoutePage(name: 'ProjectsRoute')
class ProjectsPage extends StatelessWidget {
  const ProjectsPage({super.key});

  @override
  Widget build(BuildContext context) {
    return const ProjectPanel();
  }
}
