import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flux/app.dart';
import 'package:flux/foundation/task.dart';
import 'package:flux/editor.dart';
import 'package:flux/utils/logging.dart';

class SuperTaskScreen extends ConsumerWidget {
  const SuperTaskScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final document = _loadTasksDocument();
    final composer = MutableDocumentComposer();
    final editor = createFluxDocumentEditor(
      composer: composer,
      document: document,
    );

    return Scaffold(
      appBar: AppBar(title: Text('Tasks')),
      body: SuperEditor(
        editor: editor,
        debugPaint: DebugPaintConfig(
          gestures: true,
          scrolling: true,
          layout: true,
        ),
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
    final tasks = obx.getTasks();

    final List<TaskNode> nodes = List.castFrom(
      tasks.map((task) => TaskNode.fromTask(task)).toList(),
    );

    return MutableDocument(nodes: nodes);
  }
}
