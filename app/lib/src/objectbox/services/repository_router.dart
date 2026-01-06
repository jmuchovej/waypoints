import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../things/foundation.dart';
import '../repositories/thing_repository.dart';
import '../../things/thing.entity.dart';
import 'thing_registry.dart';

/// Provider for resolver service
final repositoryRouterProvider = Provider<RepositoryRouter>((ref) {
  final thingService = ref.watch(thingRegistryProvider);
  return RepositoryRouter(thingService);
});

/// Improved resolver that integrates with repositories
class RepositoryRouter {
  RepositoryRouter(this._thingService);

  final ThingRegistry _thingService;

  // Registry of repositories by type
  final Map<String, ThingRepository> _repositories = {};

  /// Register a repository for a specific Thing type
  /// This is now done automatically by repositories
  void registerRepository<T extends BaseThing>(
    String type,
    ThingRepository<T> repository,
  ) {
    _repositories[type] = repository;
  }

  /// Resolve a single ThingEntity to its concrete type
  BaseThing? resolve(ThingEntity entity) {
    final repository = _repositories[entity.type];
    if (repository == null) throw UnregisteredEntityTypeException(entity.type);

    return repository.get(entity.id);
  }

  /// Resolve multiple ThingEntities to their concrete types
  List<BaseThing> resolveMany(List<ThingEntity> entities) {
    if (entities.isEmpty) return [];

    // Group by type for efficient batch resolution
    final grouped = <String, List<ThingEntity>>{};
    for (final entity in entities) {
      grouped.putIfAbsent(entity.type, () => []).add(entity);
    }

    // Resolve each type group
    final results = <String, BaseThing>{};
    for (final entry in grouped.entries) {
      final type = entry.key;
      final typeEntities = entry.value;
      final repository = _repositories[type];

      if (repository == null) throw UnregisteredEntityTypeException(type);

      final ids = typeEntities.map((e) => e.id).toList();
      final resolved = repository.getMany(ids);

      // Map back to UUIDs
      for (int i = 0; i < typeEntities.length && i < resolved.length; i++) {
        results[typeEntities[i].uuid] = resolved[i];
      }
    }

    // Return in original order
    return entities.map((e) => results[e.uuid]).whereType<BaseThing>().toList();
  }

  /// Resolve children of a Thing by UUID
  List<BaseThing>? resolveChildrenOf(String uuid) {
    final entity = _thingService.lookup(uuid);
    if (entity == null) return null;

    final parent = resolve(entity);
    if (parent == null) return null;

    return resolveMany((parent as ThingBehavior).children.toList());
  }

  /// Resolve typed children (used by repositories)
  List<T> resolveTypedChildren<T extends BaseThing>(
    List<ThingEntity> entities,
    String expectedType,
  ) {
    if (entities.isEmpty) return [];

    // Filter to expected type
    final typedEntities = entities
        .where((e) => e.type == expectedType)
        .toList();
    if (typedEntities.isEmpty) return [];

    final repository = _repositories[expectedType];
    if (repository == null) throw UnregisteredEntityTypeException(expectedType);

    final ids = typedEntities.map((e) => e.id).toList();
    final resolved = repository.getMany(ids);

    return resolved.whereType<T>().toList();
  }

  /// Dynamic put - routes to appropriate repository based on thing.type
  BaseThing put(BaseThing thing) {
    final repository = _repositories[thing.type];
    if (repository == null) throw UnregisteredEntityTypeException(thing.type);

    return repository.put(thing);
  }

  /// Dynamic putMany - groups by type and routes to appropriate repositories
  List<BaseThing> putMany(List<BaseThing> things) {
    if (things.isEmpty) return [];

    // Group by type for efficient batch operations
    final grouped = <String, List<BaseThing>>{};
    for (final thing in things) {
      grouped.putIfAbsent(thing.type, () => []).add(thing);
    }

    // Process each type group and maintain original order
    final resultMap = <BaseThing, BaseThing>{};
    for (final entry in grouped.entries) {
      final type = entry.key;
      final typeThings = entry.value;
      final repository = _repositories[type];

      if (repository == null) throw UnregisteredEntityTypeException(type);

      // Put each thing individually and map results
      for (final thing in typeThings) {
        resultMap[thing] = repository.put(thing);
      }
    }

    // Return results in original order
    return things.map((thing) => resultMap[thing]!).toList();
  }
}

class UnregisteredEntityTypeException implements Exception {
  UnregisteredEntityTypeException(this.type);

  final String type;

  @override
  String toString() => 'No resolver registered for entity type: $type';
}
