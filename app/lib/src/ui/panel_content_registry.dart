import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'panel_request.dart';

/// Base interface for panel content builders
abstract class PanelContentBuilder {
  /// Check if this builder can handle the given entity type
  bool canBuild(String entityType);

  /// Build the panel content widget
  Widget build(BuildContext context, String uuid, WidgetRef ref);

  /// Get the default panel configuration for this entity type
  PanelConfig getDefaultConfig() => const PanelConfig();
}

/// Registry that knows how to create panels for each Thing type
class PanelContentRegistry {
  final Map<String, PanelContentBuilder> _builders = {};

  /// Register a content builder
  void register(PanelContentBuilder builder) {
    // For universal builders that can handle any type, register with a special key
    final typeName = builder.runtimeType.toString();
    if (typeName.contains('DetailEditor')) {
      _builders['_universal'] = builder;
    } else if (typeName.contains('Task')) {
      _builders['Task'] = builder;
    } else if (typeName.contains('Project')) {
      _builders['Project'] = builder;
    } else {
      // For other builders, try to infer from class name
      _builders[typeName
              .replaceAll('PanelBuilder', '')
              .replaceAll('Builder', '')] =
          builder;
    }
  }

  /// Unregister a content builder
  void unregister(String entityType) {
    _builders.remove(entityType);
  }

  /// Get a content builder for a specific entity type
  PanelContentBuilder? getBuilder(String type) {
    // First check for specific builder
    final specificBuilder = _builders[type];
    if (specificBuilder != null && specificBuilder.canBuild(type)) {
      return specificBuilder;
    }

    // Then check universal builder
    final universalBuilder = _builders['_universal'];
    if (universalBuilder != null && universalBuilder.canBuild(type)) {
      return universalBuilder;
    }

    // Finally check all builders
    for (final builder in _builders.values) {
      if (builder.canBuild(type)) {
        return builder;
      }
    }

    return null;
  }

  /// Check if a builder is registered for an entity type
  bool hasBuilder(String type) {
    return getBuilder(type) != null;
  }

  /// Build content for a specific entity
  Widget buildContent(
    BuildContext context,
    String entityType,
    String uuid,
    WidgetRef ref,
  ) {
    final builder = getBuilder(entityType);
    if (builder == null) {
      return Center(
        child: Text('No panel builder registered for type: $entityType'),
      );
    }
    return builder.build(context, uuid, ref);
  }
}

/// Provider for the panel content registry
final panelContentRegistryProvider = Provider<PanelContentRegistry>((ref) {
  return PanelContentRegistry();
});
