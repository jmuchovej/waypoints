import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:objectbox/objectbox.dart';

import '../../editor/super_editor.dart';
import '../../infrastructure/logging.dart';
import '../../objectbox/objectbox.dart';
import '../../objectbox/objectbox.g.dart';
import '../../objectbox/providers.dart';
import '../../objectbox/services/resolver_service.dart';
import '../../objectbox/services/thing_service.dart';
import 'thing_entity.dart';

/// Base interface that all Thing entities must implement
/// This allows us to store them in typed lists instead of [List<dynamic>]
abstract class Thing {
  final children = ToMany<ThingEntity>();

  /// Direct reference to the ThingEntity registration
  /// This creates a 1:1 relationship between each Thing and its registry entry
  final thing = ToOne<ThingEntity>();

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
}

mixin ThingBehavior implements Thing {
  /// Called after the entity is saved to the database
  ThingEntity register(ThingService service) {
    try {
      final thingEntity = service.register(uuid, type, id);
      thing.target = thingEntity; // Set the ToOne relationship
      dbLog.finest('... registering new `${type}` (uuid=${uuid})');
      return thingEntity;
    } catch (e) {
      // UUID already exists - update the entity ID
      final thingEntity = service.updateID(uuid, id);
      thing.target = thingEntity; // Update the ToOne relationship
      dbLog.finest('... already registered! (uuid=${uuid})');
      return thingEntity;
    }
  }

  /// Called before the entity is deleted from the database
  void unregister(ThingService service) {
    dbLog.finest('... unregistering! (type=${type}, uuid=${uuid})');
    service.unregister(uuid);
    thing.target = null; // Clear the ToOne relationship
  }

  /// Generates a unique UUID for this entity type
  String generateUniqueUUID(ThingService service) {
    final uniqUUID = service.generateUniqueUUID();
    dbLog.finest('... generated uniq UUID (uuid=${uniqUUID})');
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
abstract class ThingRepository<T extends Thing> {
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
        parent.children.where((entity) => entity.type == type).toList();

    if (childEntities.isEmpty) return [];

    return _resolver.resolveTypedChildren<T>(childEntities, type);
  }

  /// Register this repository's resolver automatically
  void _registerResolver() {
    _resolver.registerResolver<T>(type, BoxResolver<T>());
  }

  /// Resolve a parent Thing by UUID
  Thing? _resolveParent(String uuid) {
    final entity = _thingService.lookup(uuid);
    if (entity == null) return null;
    return _resolver.resolveSingle(entity);
  }
}
