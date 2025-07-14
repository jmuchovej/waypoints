import 'package:super_editor/super_editor.dart' show MutableDocument;
import '../editor.dart' hide MutableDocument;

class OrderedSetDocument extends MutableDocument {
  OrderedSetDocument({super.nodes});

  factory OrderedSetDocument.empty() {
    final doc = OrderedSetDocument(nodes: []);
    return doc;
  }

  final Set<String> _ids = <String>{};

  @override
  void insertNodeAt(int index, DocumentNode node) {
    if (!_ids.contains(node.id)) {
      final currentIndex = toList().indexWhere((n) => n.id == node.id);
      if (currentIndex != -1) {
        moveNode(nodeId: node.id, targetIndex: index);
        return;
      }
    }

    _ids.add(node.id);
    super.insertNodeAt(index, node);
  }

  @override
  void deleteNodeAt(int index) {
    if (index < length) {
      _ids.remove(elementAt(index).id);
    }

    super.deleteNodeAt(index);
  }
}
