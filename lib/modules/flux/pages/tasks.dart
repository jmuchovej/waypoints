import 'package:flutter/material.dart';
import 'package:tempo/main.dart' show obx;
import 'package:tempo/src/editor.dart';

import 'package:tempo/src/things/task/task.dart';

class TasksScreen extends StatefulWidget {
  const TasksScreen({super.key});

  @override
  State<TasksScreen> createState() => _TasksScreenState();
}

class _TasksScreenState extends State<TasksScreen> {
  late MutableDocument _document;
  late MutableDocumentComposer _composer;
  late HoverManager _hoverManager;
  late LeaderLinkManager _linkManager;

  @override
  void initState() {
    super.initState();
    _initializeEditor();
    _hoverManager = HoverManager();
    _linkManager = LeaderLinkManager();
  }

  void _initializeEditor() {
    // Initialize document
    final nodeID = Editor.createNodeId();
    _document = MutableDocument.empty(nodeID);
    _document.deleteNode(nodeID);

    // Load tasks
    final midnight = DateTime.now().copyWith(
      hour: 0,
      minute: 0,
      second: 0,
      millisecond: 0,
      microsecond: 0,
    );
    final yesterday = midnight.subtract(const Duration(microseconds: 1));
    final tonight = yesterday.add(const Duration(days: 1));
    final tasks = obx.getTasks(
      // query: TaskEntity_.dueDate.betweenDate(yesterday, tonight),
      // order: TaskEntity_.createDate
    );
    for (final task in tasks) {
      _document.add(TaskNode.fromTask(task));
    }

    // Initialize composer
    _composer = MutableDocumentComposer();
  }

  @override
  Widget build(BuildContext context) {
    // Initialize plugins (context-dependent)
    // final motionPlugin = MotionPlugin(
    //   normalModeColor: ColorScheme.of(context).secondaryContainer,
    //   insertModeColor: ColorScheme.of(context).secondaryContainer,
    //   onNavigateToThing: (String uuid) {
    //     context.go('/thing/$uuid');
    //   },
    // );

    // Create SuperEditor with cached hover manager
    final superEditor = createSuperEditor(
      context,
      _document,
      _composer,
      hoverManager: _hoverManager,
      linkManager: _linkManager,
    );

    return Scaffold(
      // appBar: AppBar(title: const Text('Tasks')),
      backgroundColor: Theme.of(context).colorScheme.surfaceContainer,
      extendBody: true,
      body: Column(
        mainAxisAlignment: MainAxisAlignment.start,
        mainAxisSize: MainAxisSize.max,
        children: [
          Container(
            width: 640.0,
            padding: EdgeInsets.only(top: 12, bottom: 6),
            child: Text(
              "Tasks",
              style: TextTheme.of(context).displayLarge,
              textAlign: TextAlign.start,
            ),
          ),
          Expanded(child: superEditor),
        ],
      ),
    );
  }
}
