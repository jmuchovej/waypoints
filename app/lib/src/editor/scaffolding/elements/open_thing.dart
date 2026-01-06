import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:follow_the_leader/follow_the_leader.dart' hide LogNames;

import '../../../infrastructure/logging.dart';
import '../../../objectbox/services/thing_registry.dart';
import '../../../router.dart';
import '../config.dart';

// TODO: actually implement node type switching. LOL.

/// The [OpenThing] widget aims to provide a straightforward way for
/// users to toggle between various node types – this is easiest for
/// content-oriented nodes, but can also work for [ThingNode]s, too.
class OpenThing extends ConsumerStatefulWidget {
  const OpenThing(this.thingID, {super.key});

  final String thingID;

  @override
  ConsumerState<OpenThing> createState() => _OpenThingState();
}

class OpenThingElement extends ScaffoldElement {
  @override
  Widget build(BuildContext context, String nodeID, LeaderLink link) {
    return Follower.withAligner(
      link: link,
      aligner: const StaticOffsetAligner(
        offset: Offset(10, 0),
        leaderAnchor: Alignment.centerRight,
        followerAnchor: Alignment.centerLeft,
      ),
      child: OpenThing(nodeID),
    );
  }
}

class _OpenThingState extends ConsumerState<OpenThing> {
  _OpenThingState();

  void _handleTap() {
    waypointsThingTapDelegateLog.finest('tap-up!');

    // Try to determine the entity type from the UUID
    final thingService = ref.read(thingRegistryProvider);
    final thingEntity = thingService.lookup(widget.thingID);

    if (thingEntity == null) {
      throw StateError('Could not find requested thing: ${widget.thingID}');
    }

    // Open thing as panel in current context
    context.pushThingFromEntity(thingEntity, ref);
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = ColorScheme.of(context);

    return AnimatedOpacity(
      opacity: 1.0,
      duration: const Duration(milliseconds: 200),
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        child: GestureDetector(
          onTap: _handleTap,
          behavior: HitTestBehavior.opaque,
          child: Container(
            padding: const EdgeInsets.all(4),
            decoration: BoxDecoration(
              color: colorScheme.surfaceContainerHigh,
              borderRadius: BorderRadius.circular(4),
              border: Border.all(
                color: colorScheme.outline.withValues(alpha: 0.2),
              ),
            ),
            child: Icon(
              Icons.arrow_forward_rounded,
              size: 16,
              color: colorScheme.onSurface,
            ),
          ),
        ),
      ),
    );
  }
}