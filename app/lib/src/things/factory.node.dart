import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../editor/editor.dart';
import '../infrastructure/logging.dart';
import 'block/block_factories.dart';
import 'project/project_factories.dart';
import 'task/task_factories.dart';
import 'foundation.dart';
import '../objectbox/repositories/thing_repository.dart';
import 'thing.entity.dart';

/// Provider for the node factory registry with auto-registration
final nodeFactoryRegistryProvider = Provider<ThingNodeFactoryRegistry>((ref) {
  final registry = ThingNodeFactoryRegistry();

  registry.register(TaskNodeFactory(ref));
  registry.register(ProjectNodeFactory(ref));
  registry.register(BlockNodeFactory(ref));

  return registry;
});

class MissingNodeFactoryException implements Exception {
  MissingNodeFactoryException(this.type);

  final String type;

  @override
  String toString() => 'MissingNodeFactoryException: $type';
}

/// Factory interface for converting BaseThing entities to SuperEditor nodes
abstract class ThingNodeFactory<
  T extends BaseThing,
  R extends ThingRepository,
  N extends DocumentNode
> {
  ThingNodeFactory(this._ref, this._provider, this._factory);

  final Ref _ref;
  final Provider<R> _provider;
  final N Function(T entity) _factory;

  /// The entity type this factory handles
  String get type;

  /// Convert a BaseThing entity to a DocumentNode
  /// Returns null if the entity cannot be converted
  N toNodeFromEntity(T entity) => _factory(entity);

  /// Convert a ThingEntity (generic) to a DocumentNode
  /// This allows factories to work with the generic ThingEntity type
  N toNodeFromThing(ThingEntity thing) {
    if (thing.type != type) {
      throw TypeError();
    }

    final repo = _ref.read(_provider);
    final entity = repo.get(thing.id);

    if (entity == null) {
      throw StateError("`${type}` not found: ${thing.uuid} (${thing.type})");
    }

    return toNodeFromEntity(entity as T);
  }
}

/// Registry for ThingNodeFactory instances
class ThingNodeFactoryRegistry {
  final Map<String, ThingNodeFactory> _factories = {};

  /// Get all registered entity types
  List<String> get registeredTypes => _factories.keys.toList();

  /// Get a factory for a specific entity type
  ThingNodeFactory? getFactory(String entityType) {
    return _factories[entityType];
  }

  /// Check if a factory is registered for an entity type
  bool hasFactory(String entityType) {
    return _factories.containsKey(entityType);
  }

  /// Register a factory for a specific entity type
  void register<
    T extends BaseThing,
    R extends ThingRepository,
    N extends DocumentNode
  >(ThingNodeFactory<T, R, N> factory) {
    _factories[factory.type] = factory;
    waypointsLog.info('Registered node factory for ${factory.type}');
  }

  /// Convert a ThingEntity to a DocumentNode using the appropriate factory
  DocumentNode toNode(ThingEntity entity) {
    final factory = getFactory(entity.type);
    if (factory == null) {
      throw MissingNodeFactoryException(entity.type);
    }

    return factory.toNodeFromThing(entity);
  }

  DocumentNode toNodeFromEntity(BaseThing thing) {
    final factory = getFactory(thing.type);
    if (factory == null) {
      throw MissingNodeFactoryException(thing.type);
    }

    return factory.toNodeFromEntity(thing);
  }
}
