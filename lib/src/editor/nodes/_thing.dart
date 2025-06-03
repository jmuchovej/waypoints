import 'package:tempo/src/editor/super_editor.dart';

/// Base interface for DocumentNodes that represent Things (Task, Project, Activity, etc.)
///
/// Things are core domain entities that can be displayed in documents and have
/// their own dedicated editors accessible via /thing/:uuid routes.
abstract class ThingNode implements DocumentNode {
  /// The UUID of the Thing this node represents
  String get thingUuid;

  /// The type of Thing (e.g., 'task', 'project', 'activity')
  String get thingType;
}
