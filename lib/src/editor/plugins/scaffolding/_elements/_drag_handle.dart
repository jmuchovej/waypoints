import 'package:flutter/material.dart';
import 'package:follow_the_leader/follow_the_leader.dart';
import 'package:tempo/src/editor/plugins/scaffolding/_config.dart';

class DragHandle extends StatefulWidget {
  const DragHandle({super.key});

  @override
  State<DragHandle> createState() => DragHandleState();
}

class DragHandleState extends State<DragHandle> {
  DragHandleState();

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

class DragHandleElement extends ScaffoldElement {
  @override
  Widget build(BuildContext context, String nodeID, LeaderLink link) {
    return Follower.withAligner(
      link: link,
      aligner: const StaticOffsetAligner(
        offset: Offset(-10, 0),
        leaderAnchor: Alignment.centerLeft,
        followerAnchor: Alignment.centerRight,
      ),
      child: DragHandle(),
    );
  }
}
