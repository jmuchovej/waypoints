import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../infrastructure/logging.dart';
import '../../objectbox/services/resolver_service.dart';
import '../../objectbox/services/thing_service.dart';
import '../../things/thing/interface.dart';
import '../../things/thing/thing.entity.dart';
import '../../things/thing_factories.dart';
import '../../ui/panel/panel_wrapper.dart';
import '../../editor/editor.dart';
import '../../things/task/task_repository.dart';
import '404.dart';

/// 🎯 PURPOSE: Dynamic entity detail page router
/// RESPONSIBILITIES:
/// 1. UUID → Entity lookup via ThingService registry
/// 2. Entity type dispatch to appropriate panel builder
/// 3. Error handling for missing/unsupported entities
/// 4. Future: Multi-panel layout support via PanelInfo
class ThingPage extends ConsumerWidget {
  const ThingPage({super.key, required this.uuid});

  final String uuid;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // 🔍 STEP 1: UUID Resolution
    // Look up the entity from the thing service registry
    // ThingService contains lightweight metadata for all entities
    final thingService = ref.read(thingServiceProvider);
    final entity = thingService.lookup(uuid);
    tempoThingRouteLog.fine('EntityPage.build: uuid=$uuid');

    if (entity == null) {
      // UUID not found in registry - entity doesn't exist or not loaded
      return ThingNotFoundScreen(uuid: uuid);
    }

    final resolver = ref.read(resolverServiceProvider);
    final thing = resolver.resolveSingle(entity) as Thing;

    final headerFactory = ref.read(panelHeaderFactoryRegistryProvider);
    final header = headerFactory.fromThing(thing);

    final stream = _getChildren(ref, entity);

    return PanelWrapper<BaseThing>(
      header: header,
      stream: stream,
      emptyWidget: const SizedBox.shrink(),
    );
  }

  /// 🎯 PURPOSE: Universal children stream factory
  /// ARCHITECTURE: Direct children access → Type conversion
  /// 🔄 PROCESS:
  /// 1. Lookup parent in ThingService registry
  /// 2. Access children ToMany relationship directly
  /// 3. Resolve all children to their concrete types
  /// 4. Convert to Stream<BaseThing>
  Stream<BaseThing> _getChildren(WidgetRef ref, ThingEntity thing) {
    final resolver = ref.read(resolverServiceProvider);
    tempoThingRouteLog.fine(
      'Loading children for ${thing.uuid} (${thing.type}):',
    );

    final asThings = thing.children.toList();
    tempoThingRouteLog.finer(
      '  Found ${asThings.length} children on `uuid=${thing.uuid}`',
    );

    final children = resolver.resolveMany(asThings);
    for (final child in children) {
      tempoThingRouteLog.finest('  - ${child.type}: ${child.uuid}');
    }

    tempoThingRouteLog.finer(
      'Creating stream with ${children.length} children',
    );

    // For empty entities, return empty stream immediately
    // This allows PanelWrapper to transition to empty state and show editable SuperEditor
    if (children.isEmpty) {
      tempoThingRouteLog.fine(
        'No children - returning empty stream for SuperEditor display',
      );
      return const Stream.empty();
    }

    // Create a stream controller for manual control
    final controller = StreamController<BaseThing>();

    // Add all children and close
    Future.microtask(() {
      for (final child in children) {
        tempoThingRouteLog.finer(
          'Stream emitting: ${child.type} ${child.uuid}',
        );
        controller.add(child);
      }
      tempoThingRouteLog.fine(
        'Stream closing (${children.length} children emitted)',
      );
      controller.close();
    });

    return controller.stream;
  }

  /// 🎯 PURPOSE: Create editable SuperEditor for entities with no children
  /// ARCHITECTURE: Empty document → SuperEditor with placeholder text
  Widget _buildEmptyEditor(BuildContext context, WidgetRef ref) {
    // Create empty document with placeholder paragraph
    final document = MutableDocument(
      nodes: [
        ParagraphNode(
          id: Editor.createNodeId(),
          text: AttributedText('Start typing to add content...'),
        ),
      ],
    );

    final composer = MutableDocumentComposer();
    final hoverManager = HoverManager();
    final linkManager = LeaderLinkManager();
    final taskRepository = ref.read(taskRepositoryProvider);

    return Padding(
      padding: const EdgeInsets.all(16.0),
      child: createSuperEditor(
        context,
        document,
        composer,
        hoverManager: hoverManager,
        linkManager: linkManager,
        taskRepository: taskRepository,
        ref: ref,
        autofocus: true,
      ),
    );
  }
}
