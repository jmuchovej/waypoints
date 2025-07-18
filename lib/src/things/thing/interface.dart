import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:objectbox/objectbox.dart';

import '../../editor/super_editor.dart';
import '../../infrastructure/logging.dart';
import '../../objectbox/attributed_text_mixin.dart';
import '../../objectbox/objectbox.dart';
import '../../objectbox/services/resolver_service.dart';
import '../../objectbox/services/thing_service.dart';
import '../thing_factories.dart';
import 'thing.entity.dart';

/// Base interface that all Thing entities must implement
/// This allows us to store them in typed lists instead of [List<dynamic>]
abstract class BaseThing {
  /// Direct reference to the ThingEntity registration
  /// This creates a 1:1 relationship between each Thing and its registry entry
  final thing = ToOne<ThingEntity>();

  static const String key = 'thing';

  /// Support non-permanent archival
  bool get archived;

  /// When this entity was created
  DateTime get createDate;

  /// Support non-permanent deletion
  bool get deleted;

  /// ObjectBox internal ID
  int get id;

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

abstract class Thing extends BaseThing with AttributedTextEntity {
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

mixin ThingBehavior implements BaseThing {
  /// Called after the entity is saved to the database
  ThingEntity register(ThingService service) {
    final thingBox = service.store.box<ThingEntity>();

    return service.store.runInTransaction(TxMode.write, () {
      try {
        final thingEntity = service.register(uuid, type, id);
        thingEntity.plainText = getSearchableText();
        thingBox.put(thingEntity);
        thing.target = thingEntity; // Set the ToOne relationship
        tempoDBLog.finest('... registering new `${type}` (uuid=${uuid})');
        return thingEntity;
      } catch (e) {
        // UUID already exists - update the entity ID and search text
        final thingEntity = service.updateID(uuid, id);
        thingEntity.plainText = getSearchableText();
        thingBox.put(thingEntity);
        thing.target = thingEntity; // Update the ToOne relationship
        tempoDBLog.finest('... already registered! (uuid=${uuid})');
        return thingEntity;
      }
    });
  }

  /// Called before the entity is deleted from the database
  void unregister(ThingService service) {
    tempoDBLog.finest('... unregistering! (type=${type}, uuid=${uuid})');
    service.unregister(uuid);
    thing.target = null; // Clear the ToOne relationship
  }

  /// Generates a unique UUID for this entity type
  String generateUniqueUUID(ThingService service) {
    final uniqUUID = service.generateUniqueUUID();
    tempoDBLog.finest('... generated uniq UUID (uuid=${uniqUUID})');
    return uniqUUID;
  }

  /// Get the ThingEntity for this Thing
  /// Now simply returns the linked ThingEntity via ToOne relationship
  ThingEntity asThingEntity() {
    final entity = thing.target;
    if (entity == null) {
      throw StateError(
        'Thing $uuid of type $type has not been registered yet. '
        'Call register() before asThingEntity()',
      );
    }
    return entity;
  }
}

/// Base interface for DocumentNodes that represent Things (Task, Project, Activity, etc.)
///
/// Things are core domain entities that can be displayed in documents and have
/// their own dedicated editors accessible via /thing/:uuid routes.
abstract class ThingNode implements DocumentNode {
  ThingEntity get thing;
}

/// Base repository for Thing entities
/// Provides common query patterns for polymorphic Thing types
abstract class ThingRepository<T extends BaseThing> {
  ThingRepository(this._ref) {
    // Auto-register resolver for this type
    _registerResolver();
  }

  final Ref _ref;

  /// Get the ObjectBox box for the specific entity type
  Box<T> get box;
  ObjectBox get obx => _ref.read(objectBoxProvider);
  Ref get ref => _ref;

  Store get store => obx.store;

  /// Get the entity type key for this repository
  String get type;

  ResolverService get _resolver => _ref.read(resolverServiceProvider);

  ThingService get _thingService => _ref.read(thingServiceProvider);
  ThingService get ts => _thingService;

  /// Alias for getByUUID to match coding style
  T? byUUID(String uuid) => getByUUID(uuid);

  /// Get a single entity by ID
  T? get(int id) => box.get(id);

  /// Get all entities of this type
  /// WARNING: Use with caution for large datasets. Consider using filtered queries instead.
  List<T> getAll() => box.getAll();

  /// Get entity by UUID
  T? getByUUID(String uuid) {
    final entity = _thingService.lookup(uuid);
    if (entity == null || entity.type != type) return null;
    return get(entity.id);
  }

  /// Get multiple entities by IDs
  List<T?> getMany(List<int> ids) => box.getMany(ids);

  /// Save an entity and register it in the ThingEntity registry
  int put(T entity) {
    final id = box.put(entity);
    // Register the entity after saving to ensure it has a valid ID
    if (entity is ThingBehavior) {
      (entity as ThingBehavior).register(_thingService);
      // Save again to persist the ToOne relationship
      box.put(entity);
    }
    return id;
  }

  /// Save multiple entities and register them in the ThingEntity registry
  List<int> putMany(List<T> entities) {
    final ids = box.putMany(entities);
    // Register all entities after saving
    for (var i = 0; i < entities.length; i++) {
      if (entities[i] is ThingBehavior) {
        (entities[i] as ThingBehavior).register(_thingService);
      }
    }
    // Save again to persist the ToOne relationships
    box.putMany(entities);
    return ids;
  }

  /// Remove an entity and unregister it from the ThingEntity registry
  bool remove(int id) {
    final entity = box.get(id);
    if (entity != null && entity is ThingBehavior) {
      (entity as ThingBehavior).unregister(_thingService);
    }
    return box.remove(id);
  }

  /// Remove multiple entities and unregister them from the ThingEntity registry
  int removeMany(List<int> ids) {
    final entities = box.getMany(ids);
    for (final entity in entities) {
      if (entity != null && entity is ThingBehavior) {
        (entity as ThingBehavior).unregister(_thingService);
      }
    }
    return box.removeMany(ids);
  }

  /// Remove all entities of this type and unregister them from the ThingEntity registry
  int removeAll() {
    final allEntities = box.getAll();
    for (final entity in allEntities) {
      if (entity is ThingBehavior) {
        (entity as ThingBehavior).unregister(_thingService);
      }
    }
    return box.removeAll();
  }

  /// Count entities of this type
  int count() => box.count();

  /// Get children of a specific type from a parent Thing
  List<T> sameTypedChildren(String uuid) {
    final parent = _resolveParent(uuid);
    if (parent == null) return [];

    final childEntities =
        parent.thing.target!.children
            .where((entity) => entity.type == type)
            .toList();

    if (childEntities.isEmpty) return [];

    return _resolver.resolveTypedChildren<T>(childEntities, type);
  }

  List<E> children<E extends Thing>(String uuid, String type) {
    final parent = _resolveParent(uuid);
    if (parent == null) return [];

    final childEntities =
        parent.thing.target!.children
            .where((entity) => entity.type == type)
            .toList();

    if (childEntities.isEmpty) return [];

    return _resolver.resolveTypedChildren<E>(childEntities, type);
  }

  /// Register this repository's resolver automatically
  void _registerResolver() {
    _resolver.registerResolver<T>(type, BoxResolver<T>());
  }

  /// Resolve a parent Thing by UUID
  BaseThing? _resolveParent(String uuid) {
    final entity = _thingService.lookup(uuid);
    if (entity == null) return null;
    return _resolver.resolveSingle(entity);
  }

  QueryBuilder<T> query(Condition<T> condition) {
    return box.query(condition);
  }

  List<T> find(Condition<T> condition) {
    return build(query(condition)).find();
  }

  Query<T> build(QueryBuilder<T> q) => q.build();

  /// Subscribe to query changes with a callback
  StreamSubscription<Query<T>> subscription(
    QueryBuilder<T> q,
    void Function(Query<T>) callback,
  ) => q.watch(triggerImmediately: true).listen(callback);

  /// Get a stream of query results
  Stream<Query<T>> stream(Condition<T> condition) {
    return query(condition).watch(triggerImmediately: true);
  }

  /// Get a stream of query results as individual entities (flattened)
  Stream<List<T>> streamList(Condition<T> condition) {
    return stream(condition).map((q) => q.find());
  }

  /// Get a stream of query results as individual entities (flattened)
  Stream<T> streamEntities(Condition<T> condition) {
    return stream(condition).asyncExpand((q) async* {
      final results = q.find();
      for (final result in results) {
        yield result;
      }
    });
  }

  /// Get a stream of an entity by UUID
  /// Note: This streams all entities and filters in memory. For better performance,
  /// consider using streamList with a specific condition if available.
  Stream<T?> streamByUUID(String uuid) {
    // Create a stream of all entities since we can't query by UUID directly
    return box.query().watch(triggerImmediately: true).map((query) {
      final entities = query.find();
      try {
        return entities.firstWhere((entity) => entity.uuid == uuid);
      } catch (e) {
        return null; // No entity with this UUID found
      }
    });
  }

  /// Get a stream count of entities matching condition
  Stream<int> streamCount(Condition<T> condition) {
    return stream(condition).map((entities) => entities.count());
  }

  Condition<T> dateBetween(
    QueryDateNanoProperty<T> property, {
    required DateTime start,
    required DateTime end,
  }) {
    final startMillis = start.millisecondsSinceEpoch;
    final endMillis = end.millisecondsSinceEpoch;

    return property.between(startMillis, endMillis);
  }

  // TODO lift `searchTitle` (from `TaskRepo`) into `ThingRepo`

  /// Convert an entity to a DocumentNode using the registered node factory
  Future<DocumentNode?> toNode(T entity) async {
    final registry = _ref.read(nodeFactoryRegistryProvider);
    final factory = registry.getFactory(type);

    if (factory == null) {
      tempoLog.warning('No node factory registered for entity type: $type');
      return null;
    }

    // Convert to ThingEntity for the factory
    if (entity is ThingBehavior) {
      final thingEntity = (entity as ThingBehavior).asThingEntity();
      return factory.toNodeFromThing(thingEntity);
    }

    tempoLog.warning('Entity does not implement ThingBehavior: $type');
    return null;
  }

  /// Convert a ThingEntity to a DocumentNode (for children)
  Future<DocumentNode?> toNodeFromThingEntity(ThingEntity thingEntity) async {
    final registry = _ref.read(nodeFactoryRegistryProvider);
    return registry.toNode(thingEntity);
  }
}
