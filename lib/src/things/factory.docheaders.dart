import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../infrastructure/logging.dart';
import '../ui/widgets/panel/panel_header.dart';
import 'project/project_factories.dart';
import 'task/task_factories.dart';
import 'thing.entity.dart';
import 'foundation.dart';
import 'repository.dart';

final panelHeaderFactoryRegistryProvider = Provider<PanelHeaderFactoryRegistry>(
  (ref) {
    final registry = PanelHeaderFactoryRegistry();

    registry.register(TaskHeaderFactory(ref));
    registry.register(ProjectHeaderFactory(ref));

    return registry;
  },
);

class MissingHeaderFactoryException implements Exception {
  MissingHeaderFactoryException(this.type);

  final String type;

  @override
  String toString() => 'MissingHeaderFactoryException: $type';
}

/// Registry for ThingNodeFactory instances
class PanelHeaderFactoryRegistry {
  final Map<String, ThingPanelHeaderFactory> _factories = {};

  /// Get all registered entity types
  List<String> get registeredTypes => _factories.keys.toList();

  PanelHeaderMixin fromThing(Thing thing) {
    final factory = getFactory(thing.type);
    if (factory == null) {
      throw MissingHeaderFactoryException(thing.type);
    }

    return factory.buildPanelHeaderFromEntity(thing);
  }

  /// Convert a ThingEntity to a DocumentNode using the appropriate factory
  PanelHeaderMixin fromThingEntity(ThingEntity entity) {
    final factory = getFactory(entity.type);
    if (factory == null) {
      throw MissingHeaderFactoryException(entity.type);
    }

    return factory.buildPanelHeader(entity);
  }

  /// Get a factory for a specific entity type
  ThingPanelHeaderFactory? getFactory(String entityType) {
    return _factories[entityType];
  }

  /// Check if a factory is registered for an entity type
  bool hasFactory(String entityType) {
    return _factories.containsKey(entityType);
  }

  /// Register a factory for a specific entity type
  void register<T extends Thing, R extends ThingRepository>(
    ThingPanelHeaderFactory<T, R> factory,
  ) {
    _factories[factory.type] = factory;
    tempoLog.info('Registered node factory for ${factory.type}');
  }
}

abstract class ThingPanelHeaderFactory<
  T extends Thing,
  R extends ThingRepository
> {
  ThingPanelHeaderFactory(this._ref, this._provider, this._factory);

  final Ref _ref;
  final Provider<R> _provider;
  final PanelHeaderMixin Function(T thing) _factory;

  /// The entity type this factory handles
  String get type;

  /// Convert a ThingEntity (generic) to a DocumentNode
  /// This allows factories to work with the generic ThingEntity type
  PanelHeaderMixin buildPanelHeader(ThingEntity thing) {
    if (thing.type != type) {
      throw TypeError();
    }

    final repo = _ref.read(_provider);
    final entity = repo.get(thing.id);

    if (entity == null) {
      throw StateError("`${type}` not found: ${thing.uuid} (${thing.type})");
    }

    return buildPanelHeaderFromEntity(entity as T);
  }

  /// Convert a BaseThing entity to a DocumentNode
  /// Returns null if the entity cannot be converted
  PanelHeaderMixin buildPanelHeaderFromEntity(T entity) => _factory(entity);
}
