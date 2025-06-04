import 'package:flutter/material.dart';
import 'package:follow_the_leader/follow_the_leader.dart';
import 'package:tempo/src/editor/scaffolding/_config.dart';
import 'package:tempo/src/things/things.dart' show ThingNode;

// TODO: actually implement node type switching. LOL.

/// The [SwitchNodeType] widget aims to provide a straightforward way for
/// users to toggle between various node types – this is easiest for
/// content-oriented nodes, but can also work for [ThingNode]s, too.
class SwitchNodeType extends StatefulWidget {
  const SwitchNodeType({super.key});

  @override
  State<SwitchNodeType> createState() => SwitchNodeTypeState();
}

class SwitchNodeTypeState extends State<SwitchNodeType> {
  SwitchNodeTypeState();

  @override
  Widget build(BuildContext context) {
    return AnimatedOpacity(
      opacity: 1.0,
      duration: Duration(milliseconds: 200),
      child: MouseRegion(
        cursor: SystemMouseCursors.grab,
        child: Container(
          padding: const EdgeInsets.all(4),
          decoration: BoxDecoration(
            color: ColorScheme.of(context).surface,
            borderRadius: BorderRadius.circular(4.0),
            border: Border.all(
              color: ColorScheme.of(context).outline.withValues(alpha: 0.2),
            ),
          ),
          child: Icon(
            Icons.drag_indicator,
            size: 16,
            color: ColorScheme.of(context).onSurface,
          ),
        ),
      ),
    );
  }
}

class SwitchNodeTypeElement extends ScaffoldElement {
  @override
  Widget build(BuildContext context, String nodeID, LeaderLink link) {
    return Follower.withAligner(
      link: link,
      aligner: const StaticOffsetAligner(
        offset: Offset(20, 0),
        leaderAnchor: Alignment.centerRight,
        followerAnchor: Alignment.centerLeft,
      ),
      child: SwitchNodeType(),
    );
  }
}
