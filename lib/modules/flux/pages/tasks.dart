import 'package:flutter/material.dart';
import 'package:auto_route/auto_route.dart';
import 'package:tempo/main.dart' show obx;
import 'package:tempo/src/editor.dart';
import 'package:tempo/src/infrastructure.dart';

import 'package:tempo/src/things/task.dart';

@RoutePage()
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
    fluxLog.info('📋 TasksScreen.initState() called');
    try {
      // Initialize managers first
      _hoverManager = HoverManager();
      fluxLog.info('📋 TasksScreen.initState() - hover manager created');
      _linkManager = LeaderLinkManager();
      fluxLog.info('📋 TasksScreen.initState() - link manager created');

      // Initialize editor after managers are ready
      _initializeEditor();
      fluxLog.info('📋 TasksScreen.initState() - editor initialized');

      fluxLog.info('📋 TasksScreen.initState() completed successfully');
    } catch (e, stackTrace) {
      fluxLog.severe('📋 Error in TasksScreen.initState(): $e', e, stackTrace);
      rethrow;
    }
  }

  void _initializeEditor() {
    fluxLog.info('📋 _initializeEditor() called');
    try {
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
      // Debug: Check total task count and parent relationships
      final allTasks = obx.getTasks();
      fluxLog.info('📋 Total tasks in database: ${allTasks.length}');

      // Debug: Check parent relationships
      int tasksWithParents = 0;
      int tasksWithoutParents = 0;
      for (final task in allTasks) {
        if (task.parent.hasValue) {
          tasksWithParents++;
        } else {
          tasksWithoutParents++;
        }
      }
      fluxLog.info('📋 Tasks with parents: $tasksWithParents, without parents: $tasksWithoutParents');

      // Load only top-level tasks (tasks without parents) - manual filtering for now
      // TODO: Fix ObjectBox query syntax for null relations
      final topLevelTasks = allTasks.where((task) => !task.parent.hasValue).toList();
      fluxLog.info('📋 Loaded ${topLevelTasks.length} top-level tasks via manual filtering');

      // Process each top-level task and its hierarchy
      for (int i = 0; i < topLevelTasks.length; i++) {
        final task = topLevelTasks[i];
        try {
          fluxLog.info('📋 Processing top-level task ${i + 1}/${topLevelTasks.length}: ${task.title.text}');
          _addTaskWithSubtasks(task);
          fluxLog.info('📋 Added task hierarchy for: ${task.title.text}');
        } catch (e, stackTrace) {
          fluxLog.severe('📋 Error processing task ${i + 1} (${task.title.text}): $e', e, stackTrace);
          // Continue processing other tasks instead of failing completely
        }
      }

      fluxLog.info('📋 Finished processing all tasks');

      // Initialize composer
      fluxLog.info('📋 Initializing MutableDocumentComposer');
      _composer = MutableDocumentComposer();
      fluxLog.info('📋 _initializeEditor() completed successfully');
    } catch (e, stackTrace) {
      fluxLog.severe('📋 Error in _initializeEditor(): $e', e, stackTrace);
      rethrow;
    }
  }

  @override
  void dispose() {
    super.dispose();
  }

  /// Adds only top-level tasks to the document (no subtasks shown directly).
  void _addTaskWithSubtasks(TaskEntity task) {
    // Add the parent task
    final taskNode = TaskNode.fromTask(task);
    _document.add(taskNode);

    // Don't add subtasks to the document - they'll be shown as progress pills instead
  }

  @override
  Widget build(BuildContext context) {
    fluxLog.info('📋 TasksScreen.build() called');

    // Initialize plugins (context-dependent)
    // final motionPlugin = MotionPlugin(
    //   normalModeColor: ColorScheme.of(context).secondaryContainer,
    //   insertModeColor: ColorScheme.of(context).secondaryContainer,
    //   onNavigateToThing: (String uuid) {
    //     context.go('/thing/$uuid');
    //   },
    // );

    // Create SuperEditor with cached hover manager
    Widget superEditor;
    try {
      superEditor = createSuperEditor(
        context,
        _document,
        _composer,
        hoverManager: _hoverManager,
        linkManager: _linkManager,
      );
      fluxLog.info('📋 SuperEditor created successfully');
    } catch (e, stackTrace) {
      fluxLog.severe('📋 Error creating SuperEditor: $e', e, stackTrace);
      superEditor = Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(Icons.error, size: 48, color: Colors.red),
            const SizedBox(height: 16),
            Text('Error creating editor: $e'),
          ],
        ),
      );
    }

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
