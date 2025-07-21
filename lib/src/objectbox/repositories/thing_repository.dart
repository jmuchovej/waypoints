import 'dart:async';

import 'package:collection/collection.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../editor/editor.dart';
import '../../infrastructure/logging.dart';
import '../objectbox.dart';
import '../objectbox.g.dart';
import '../services/repository_router.dart';
import '../services/thing_registry.dart';
import '../../things/factory.node.dart';
import '../../things/foundation.dart';
import '../../things/thing.entity.dart';

/// Foundational repository for each [BaseThing]. This provides common query
/// patterns shared by each [BaseThing] implementer.
///
/// **Each [BaseThing] implementer should extend [ThingRepository]!**
abstract class ThingRepository<T extends BaseThing> {
  /// Using the incoming [Ref], register this [ThingRepository] with [RepositoryRouter].
  ThingRepository(this._ref) {
    // Auto-register this repository for dynamic routing
    _repoRouter.registerRepository<T>(type, this);
  }

  final Ref _ref;

  Ref get ref => _ref;

  ObjectBox get obx => _ref.read(objectBoxProvider);
  Store get store => obx.store;
  /// Get the ObjectBox box for the specific entity type
  Box<T> get box => Box<T>(store);

  ThingRegistry get ts => _registry;

  /// Get the entity type key for this repository
  String get type;

  RepositoryRouter get _repoRouter => _ref.read(repositoryRouterProvider);
  ThingRegistry get _registry => _ref.read(thingRegistryProvider);

  Query<T> build(QueryBuilder<T> q) => q.build();

  /// Alias for getByUUID to match coding style
  T? byUUID(String uuid) => getByUUID(uuid);

  List<E> children<E extends Thing>(String uuid, String type) {
    final parent = _resolveParent(uuid);
    if (parent == null) return [];

    final childEntities =
        (parent as ThingBehavior).children
            .where((entity) => entity.type == type)
            .toList();

    if (childEntities.isEmpty) return [];

    return _repoRouter.resolveTypedChildren<E>(childEntities, type);
  }

  /// Count entities of this type
  int count() => box.count();

  Condition<T> dateBetween(
    QueryDateNanoProperty<T> property, {
    required DateTime start,
    required DateTime end,
  }) {
    final startMillis = start.millisecondsSinceEpoch;
    final endMillis = end.millisecondsSinceEpoch;

    return property.between(startMillis, endMillis);
  }

  List<T> find(Condition<T> condition) {
    return build(query(condition)).find();
  }

  /// Get a single entity by ID
  T? get(int id) => getMany([id]).firstOrNull;

  /// Get all entities of this type
  /// WARNING: Use with caution for large datasets. Consider using filtered queries instead.
  List<T> getAll() => box.getAll();

  /// Get entity by UUID
  T? getByUUID(String uuid) {
    final entity = ts.lookup(uuid);
    if (entity == null || entity.type != type) return null;
    return get(entity.id);
  }

  /// Get multiple entities by IDs
  List<T> getMany(List<int> ids) =>
      box.getMany(ids).where((result) => result != null).cast<T>().toList();

  T put(T entity) {
    final savedID = box.put(entity);
    final saved = box.get(savedID)!;

    final BaseThing(uuid: uuid, type: type, id: id) = saved;
    final existing = ts.lookup(uuid);

    if (!saved.thingRef.hasValue) {
      final thing = ThingEntity(id: id, type: type, uuid: uuid)
        ..plainText = saved.getSearchableText();
      if (existing != null) {
        thing.registryID = existing.registryID;
      }
      saved.thingRef.target = thing;
    }

    final thingBox = ts.store.box<ThingEntity>();
    final thingID = thingBox.put((saved as ThingBehavior).thing);
    final thing = thingBox.get(thingID)!;
    saved.thingRef.target = thing;

    tempoDBLog.finest(
      "... `thing` =?= ${(saved as ThingBehavior).thing == thing}",
    );

    final latestID = box.put(saved);
    return box.get(latestID)!;
  }

  /// Save an entity and register it in the ThingEntity registry
  Future<T> putAsync(T entity) async {
    final saved = await box.putAndGetAsync(entity);
    final BaseThing(uuid: uuid, type: type, id: id) = saved;
    final existing = ts.lookup(uuid);
    if (!saved.thingRef.hasValue) {
      final thing = ThingEntity(id: id, type: type, uuid: uuid)
        ..plainText = saved.getSearchableText();
      if (existing != null) {
        thing.registryID = existing.registryID;
      }
      saved.thingRef.target = thing;
    }
    final thing = await ts.store.box<ThingEntity>().putAndGetAsync(
      (saved as ThingBehavior).thing,
    );
    saved.thingRef.target = thing;
    tempoDBLog.finest(
      "... `thing` =?= ${(saved as ThingBehavior).thing == thing}",
    );
    return box.putAndGetAsync(saved);
  }

  QueryBuilder<T> query(Condition<T> condition) {
    return box.query(condition);
  }

  /// Remove an entity and unregister it from the ThingEntity registry
  int remove(int id) => removeMany([id]);

  /// Remove all entities of this type and unregister them from the ThingEntity registry
  int removeAll() => removeMany(getAll().map((e) => e.id).toList());

  /// Remove multiple entities and unregister them from the ThingEntity registry
  int removeMany(List<int> ids) {
    final entities = getMany(ids);
    for (final entity in entities) {
      (entity as ThingBehavior).unregister(_registry);
    }
    return box.removeMany(ids);
  }

  /// Get children of a specific type from a parent Thing
  List<T> sameTypedChildren(String uuid) {
    final parent = _resolveParent(uuid);
    if (parent == null) return [];

    final childEntities =
        (parent as ThingBehavior).children
            .where((entity) => entity.type == type)
            .toList();

    if (childEntities.isEmpty) return [];

    return _repoRouter.resolveTypedChildren<T>(childEntities, type);
  }

  /// Get a stream of query results
  Stream<Query<T>> stream(Condition<T> condition) {
    return query(condition).watch(triggerImmediately: true);
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

  /// Get a stream of query results as individual entities (flattened)
  Stream<T> streamEntities(Condition<T> condition) {
    return stream(condition).asyncExpand((q) async* {
      final results = q.find();
      for (final result in results) {
        yield result;
      }
    });
  }

  /// Get a stream of query results as individual entities (flattened)
  Stream<List<T>> streamList(Condition<T> condition) {
    return stream(condition).map((q) => q.find());
  }

  /// Subscribe to query changes with a callback
  StreamSubscription<Query<T>> subscription(
    QueryBuilder<T> q,
    void Function(Query<T>) callback,
  ) => q.watch(triggerImmediately: true).listen(callback);

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
      final thingEntity = (entity as ThingBehavior).thing;
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

  // TODO lift `searchTitle` (from `TaskRepo`) into `ThingRepo`

  /// Resolve a parent Thing by UUID
  BaseThing? _resolveParent(String uuid) {
    final entity = _registry.lookup(uuid);
    if (entity == null) return null;
    return _repoRouter.resolve(entity);
  }
}
