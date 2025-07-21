import 'package:collection/collection.dart';
import 'package:equatable/equatable.dart';

import '../editor/editor.dart';
import '../infrastructure/logging.dart';
import '../objectbox/attributed_text_mixin.dart';
import '../objectbox/objectbox.g.dart';
import '../objectbox/services/thing_registry.dart';
import 'thing.entity.dart';

/// Base interface that all Thing entities must implement
/// This allows us to store them in typed lists instead of [List<dynamic>]
abstract class BaseThing {
  /// Direct reference to the ThingEntity registration
  /// This creates a 1:1 relationship between each Thing and its registry entry
  final thingRef = ToOne<ThingEntity>();

  static const String key = 'thing';

  /// Support non-permanent archival
  bool get archived;

  /// When this entity was created
  DateTime get createDate;

  /// Support non-permanent deletion
  bool get deleted;

  /// ObjectBox internal ID
  @Id()
  int id = 0;

  /// When this entity was last modified
  DateTime get modifyDate;

  /// Entity type identifier ('task', 'project', 'activity', etc.)
  String get type;

  /// Unique identifier across all Thing types
  String get uuid;

  /// Get searchable text content for this entity
  /// This content will be stored in the ThingEntity registry for unified search
  String? getSearchableText();
}

abstract class Thing extends BaseThing
    with ThingBehavior, AttributedTextEntity {
  String titleJson = "";

  @Transient()
  AttributedText get title => getAttributedText(titleJson);
  set title(AttributedText value) {
    setAttributedText(value, (json) => titleJson = json);
  }

  @override
  String getSearchableText() => title.toPlainText();
}

abstract class ThingHistory {
  /// Support non-permanent archival
  bool? get archived;

  /// When this entity was created
  DateTime timestamp = DateTime.now();

  /// Support non-permanent deletion
  bool? get deleted;

  /// ObjectBox internal ID
  int get id;
}

abstract class ComparableThing extends Equatable implements BaseThing {
  Map<String, Object?> get propsMap;

  @override
  List<Object?> get props => propsMap.values.toList();

  @override
  bool get stringify => true;

  Map<String, dynamic> diff(covariant ComparableThing other) {
    final differences = <String, dynamic>{};

    propsMap.forEach((key, prev) {
      final curr = other.propsMap[key];
      if (prev != curr) {
        differences[key] = {'prev': prev, 'curr': curr};
      }
    });

    return differences;
  }
}

mixin ThingBehavior implements BaseThing {
  ThingEntity get thing {
    final target = thingRef.target;
    if (target == null) {
      throw StateError('`thingRef` is null!');
    }
    return target;
  }

  // Sugar to forward methods since we need this all over the place!
  List<ThingEntity> get children => thing.children;
  ThingEntity? get parent => thing.parent;

  /// Called after the entity is saved to the database
  ThingEntity register(ThingRegistry service) {
    final thingBox = service.store.box<ThingEntity>();

    return service.store.runInTransaction(TxMode.write, () {
      ThingEntity thingEntity;

      try {
        thingEntity = service.register(uuid, type, id);
        tempoDBLog.finest('... registering new `${type}` (uuid=${uuid})');
      } catch (e) {
        tempoDBLog.finest('... already registered! (uuid=${uuid})');
        thingEntity = service.updateID(uuid, id);
        tempoDBLog.finest('...... ${e}');
      }

      thingEntity.plainText = getSearchableText();
      thingRef.target = thingEntity; // Set the ToOne relationship
      thingBox.put(thingEntity);
      tempoDBLog.finest('... `thingRef.target` =?= ${thingRef.target}');
      return thingEntity;
    });
  }

  /// Called before the entity is deleted from the database
  void unregister(ThingRegistry service) {
    tempoDBLog.finest('... unregistering! (type=${type}, uuid=${uuid})');
    service.unregister(uuid);
    thingRef.target = null; // Clear the ToOne relationship
  }

  /// Generates a unique UUID for this entity type
  String generateUniqueUUID(ThingRegistry service) {
    final uniqUUID = service.generateUniqueUUID();
    tempoDBLog.finest('... generated uniq UUID (uuid=${uniqUUID})');
    return uniqUUID;
  }
}

/// Base interface for DocumentNodes that represent Things (Task, Project, Activity, etc.)
///
/// Things are core domain entities that can be displayed in documents and have
/// their own dedicated editors accessible via /thing/:uuid routes.
abstract class ThingNode implements DocumentNode {
  ThingEntity get thing;
}

extension BaseThingListExtensions on List<BaseThing> {
}

extension BaseThingIterableExtensions on Iterable<BaseThing> {
  Iterable<ThingEntity> get things => map((t) => (t as ThingBehavior).thing);

  Iterable<ThingEntity?> get parents => map((t) => (t as ThingBehavior).parent);
  Iterable<ThingEntity> get children => map((t) => (t as ThingBehavior).children).flattened;
}
