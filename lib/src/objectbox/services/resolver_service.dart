import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../things/thing/interface.dart';
import '../objectbox.dart';
import '../objectbox.g.dart';
import '../../things/thing/thing_entity.dart';
import '../providers.dart';
import 'thing_service.dart';

/// Provider for resolver service
final resolverServiceProvider = Provider<ResolverService>((ref) {
  final obx = ref.watch(objectBoxProvider);
  final thingService = ref.watch(thingServiceProvider);
  return ResolverService(obx, thingService);
});

/// Improved resolver that integrates with repositories
class ResolverService {
  ResolverService(this._obx, this._thingService);

  final ObjectBox _obx;
  final ThingService _thingService;

  // Registry of resolver functions by type
  final Map<String, ThingResolver> _resolvers = {};

  /// Register a resolver for a specific Thing type
  /// This is now done automatically by repositories
  void registerResolver<T extends Thing>(
    String type,
    ThingResolver<T> resolver,
  ) {
    _resolvers[type] = resolver;
  }

  /// Resolve a single ThingEntity to its concrete type
  Thing? resolveSingle(ThingEntity entity) {
    final resolver = _resolvers[entity.type];
    if (resolver == null) throw UnregisteredEntityTypeException(entity.type);

    final results = resolver.resolveMany(_obx, [entity.id]);
    return results.firstOrNull;
  }

  /// Resolve multiple ThingEntities to their concrete types
  List<Thing> resolveMany(List<ThingEntity> entities) {
    if (entities.isEmpty) return [];

    // Group by type for efficient batch resolution
    final grouped = <String, List<ThingEntity>>{};
    for (final entity in entities) {
      grouped.putIfAbsent(entity.type, () => []).add(entity);
    }

    // Resolve each type group
    final results = <String, Thing>{};
    for (final entry in grouped.entries) {
      final type = entry.key;
      final typeEntities = entry.value;
      final resolver = _resolvers[type];

      if (resolver == null) throw UnregisteredEntityTypeException(type);

      final ids = typeEntities.map((e) => e.id).toList();
      final resolved = resolver.resolveMany(_obx, ids);

      // Map back to UUIDs
      for (int i = 0; i < typeEntities.length && i < resolved.length; i++) {
        if (resolved[i] != null) {
          results[typeEntities[i].uuid] = resolved[i]!;
        }
      }
    }

    // Return in original order
    return entities.map((e) => results[e.uuid]).whereType<Thing>().toList();
  }

  /// Resolve children of a Thing by UUID
  List<Thing>? resolveChildrenOf(String uuid) {
    final entity = _thingService.lookup(uuid);
    if (entity == null) return null;

    final parent = resolveSingle(entity);
    if (parent == null) return null;

    return resolveMany(parent.children.toList());
  }

  /// Resolve typed children (used by repositories)
  List<T> resolveTypedChildren<T extends Thing>(
    List<ThingEntity> entities,
    String expectedType,
  ) {
    if (entities.isEmpty) return [];

    // Filter to expected type
    final typedEntities =
        entities.where((e) => e.type == expectedType).toList();
    if (typedEntities.isEmpty) return [];

    final resolver = _resolvers[expectedType];
    if (resolver == null) throw UnregisteredEntityTypeException(expectedType);

    final ids = typedEntities.map((e) => e.id).toList();
    final resolved = resolver.resolveMany(_obx, ids);

    return resolved.whereType<T>().toList();
  }
}

/// Base resolver interface
abstract class ThingResolver<T extends Thing> {
  List<T?> resolveMany(ObjectBox obx, List<int> ids);
}

/// Generic resolver implementation that uses [Box<T>]
class BoxResolver<T extends Thing> implements ThingResolver<T> {
  const BoxResolver();

  @override
  List<T?> resolveMany(ObjectBox obx, List<int> ids) {
    return Box<T>(obx.store).getMany(ids);
  }
}

class UnregisteredEntityTypeException implements Exception {
  UnregisteredEntityTypeException(this.type);

  final String type;

  @override
  String toString() => 'No resolver registered for entity type: $type';
}
