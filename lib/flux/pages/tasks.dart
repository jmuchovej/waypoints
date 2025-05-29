import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:tempo/main.dart' show obx;
import 'package:tempo/src/editor/plugins/motion/motion.dart';
import 'package:tempo/src/editor/plugins/node_metadata/node_metadata.dart';
import 'package:tempo/src/editor/plugins/task/task.dart';
import 'package:tempo/flux/editor.dart';
import 'package:tempo/src/database/objectbox.g.dart';
import 'package:tempo/src/database/objectbox/models/task.dart';
import 'package:tempo/src/infrastructure/router.dart';

class TasksScreen extends StatelessWidget {
  const TasksScreen({super.key});

  @override
  Widget build(BuildContext context) {
    // EditorState editorState = EditorState.blank(withInitialText: false);
    final nodeID = Editor.createNodeId();
    final document = MutableDocument.empty(nodeID);
    document.deleteNode(nodeID);
    final midnight = DateTime.now().copyWith(hour: 0, minute: 0, second: 0, millisecond:  0, microsecond: 0);
    final yesterday = midnight.subtract(const Duration(microseconds: 1));
    final tonight   = yesterday.add(const Duration(days: 1));
    final tasks = obx.getTasks(
      query: TaskEntity_.dueDate.betweenDate(yesterday, tonight),
      order: TaskEntity_.createDate
    );
    for (final task in tasks) {
      document.add(TaskNode.fromTask(task));
    }
    // for (final (index, task) in tasks.indexed) {
    //   editorState.document.insert([index], [todoListNode(checked: task.status.isCompleted, text: task.name)]);
    // }
    final composer = MutableDocumentComposer();
    final nodeMetadataPlugin = NodeMetadataPlugin();
    final motionPlugin = MotionPlugin(
      normalModeColor: ColorScheme.of(context).secondaryContainer,
      insertModeColor: ColorScheme.of(context).secondaryContainer,
    );
    final taskPlugin = TaskPlugin(
      onTaskTap: (TaskEntity task) {
        context.go('/thing/${task.uuid}');
      },
    );
    final markdownPlugin = MarkdownInlineUpstreamSyntaxPlugin();
    final plugins = {nodeMetadataPlugin, motionPlugin, markdownPlugin, taskPlugin};

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
            child:
              Text("Tasks", style: TextTheme.of(context).displayLarge, textAlign: TextAlign.start,),
          ),
          Expanded(child: createSuperEditor(context, document, composer, plugins))
        ],
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: () => context.go('/task/new'),
        child: const Icon(Icons.add),
      ),
    );
  }
}
