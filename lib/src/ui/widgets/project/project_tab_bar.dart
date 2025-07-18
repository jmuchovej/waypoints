import 'package:flutter/material.dart';

class ProjectTabBar extends StatelessWidget {
  const ProjectTabBar({super.key, required this.tabController});

  final TabController tabController;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 24.0),
      child: Align(
        alignment: Alignment.centerLeft,
        child: TabBar(
          controller: tabController,
          isScrollable: true,
          tabAlignment: TabAlignment.start,
          tabs: const [Tab(text: 'Notes'), Tab(text: 'Tasks')],
        ),
      ),
    );
  }
}
