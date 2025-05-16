import 'package:flutter/material.dart';
import 'package:flux/foundation/task.dart';
import 'package:flux/objectbox.dart';
import 'package:flux/ui/editor.dart';

class SuperTaskScreen extends StatelessWidget {
  const SuperTaskScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final document = _loadTasksDocument();
    final composer = MutableDocumentComposer();
    final editor = createDefaultDocumentEditor(
      composer: composer,
      document: document,
    );

    return Scaffold(
      appBar: AppBar(title: Text('Tasks')),
      body: SuperEditor(
        editor: editor,
        componentBuilders: [
          TaskComponentBuilder(editor),
          // ProjectComponentBuilder(),
          // ActivityComponentBuilder(),
          ...defaultComponentBuilders,
        ],
        selectionStyle: SelectionStyles(
          selectionColor: Colors.blue.withValues(alpha: 0.3),
        ),
      ),
    );
  }

  MutableDocument _loadTasksDocument() {
    final tasks = ObjectBox.taskBox().getAll();

    final List<TaskNode> nodes = List.castFrom(
      tasks.map((task) => TaskNode.fromTask(task)).toList(),
    );

    return MutableDocument(nodes: nodes);
  }
}
