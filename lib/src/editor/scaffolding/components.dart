import 'package:flutter/material.dart';
import 'package:follow_the_leader/follow_the_leader.dart';

import '../../infrastructure/logging.dart';
import '../../infrastructure/platform.dart';
import '../super_editor.dart';
import 'managers/hover.dart';
import 'managers/link.dart';

class ScaffoldedComponentBuilder extends ComponentBuilder {
  ScaffoldedComponentBuilder({
    required this.innerBuilder,
    required this.linkManager,
    required this.hoverManager,
    required this.composer,
  });

  final ComponentBuilder innerBuilder;
  final LeaderLinkManager linkManager;
  final HoverManager hoverManager;
  final DocumentComposer composer;

  @override
  Widget? createComponent(
    SingleColumnDocumentComponentContext componentContext,
    SingleColumnLayoutComponentViewModel componentViewModel,
  ) {
    final child = innerBuilder.createComponent(
      componentContext,
      componentViewModel,
    );
    if (child == null) return null;

    final nodeId = componentViewModel.nodeId;

    return ListenableBuilder(
      listenable: Listenable.merge([composer.selectionNotifier, hoverManager]),
      builder: (context, _) {
        // Check if this node should be highlighted
        bool isSelected = false;
        bool isHovered = false;

        final selection = composer.selection;
        if (selection != null) {
          isSelected =
              selection.base.nodeId == nodeId ||
              selection.extent.nodeId == nodeId;
        }

        isHovered = hoverManager.isHovered(nodeId);

        final colorScheme = ColorScheme.of(context);
        // Determine highlight color
        Color? highlightColor;
        if (isSelected) {
          highlightColor = colorScheme.primaryContainer.withValues(alpha: 0.3);
        } else if (isHovered) {
          highlightColor = colorScheme.surfaceContainerHighest;
        } else {
          highlightColor = colorScheme.surfaceContainer;
        }

        // Wrap with highlight if needed
        final component = _HighlightWrapper(
          color: highlightColor,
          child: child,
        );

        // Wrap with Leader for scaffolding elements
        return _ScaffoldedComponentWrapper(
          nodeId: nodeId,
          linkManager: linkManager,
          child: component,
        );
      },
    );
  }

  @override
  SingleColumnLayoutComponentViewModel? createViewModel(
    Document document,
    DocumentNode node,
  ) {
    return innerBuilder.createViewModel(document, node);
  }
}

/// Paints a full-width highlight that extends to the screen edges.
class _FullWidthHighlight extends StatefulWidget {
  const _FullWidthHighlight({required this.color, required this.padding});

  final Color color;
  final double padding;

  @override
  State<_FullWidthHighlight> createState() => _FullWidthHighlightState();
}

class _FullWidthHighlightState extends State<_FullWidthHighlight> {
  double? _leftOffset;

  @override
  Widget build(BuildContext context) {
    // Return empty until we have the offset
    if (_leftOffset == null) {
      return const SizedBox();
    }

    final width = MediaQuery.of(context).size.width - (2 * widget.padding);
    final textSize = TextTheme.of(context).bodyLarge!.fontSize!;

    return OverflowBox(
      alignment: Alignment.centerLeft,
      // minWidth: width,
      // maxWidth: width,
      child: Transform.translate(
        // offset: Offset(_leftOffset! + widget.padding, 0),
        offset: Offset(0, 0),
        child: Container(
          width: width,
          decoration: BoxDecoration(
            color: widget.color,
            borderRadius: BorderRadius.circular(textSize * 0.5),
          ),
        ),
      ),
    );
  }

  @override
  void initState() {
    super.initState();
    // NOTE: 06/03/2025 – This could cause unnecessary perf issues. IF it
    // does, refactor this.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _calculateOffset();
    });
  }

  void _calculateOffset() {
    final box = context.findRenderObject() as RenderBox?;
    if (box == null || !box.hasSize) {
      pluginScaffoldLog.warning('RenderBox not available for highlight');
      return;
    }

    final offset = box.localToGlobal(Offset.zero);
    setState(() {
      _leftOffset = -offset.dx;
    });

    pluginScaffoldLog.finest('Calculated highlight offset: $_leftOffset');
  }
}

/// A widget that wraps a child with a full-width highlight background.
///
/// The highlight extends from the left edge to the right edge of the screen,
/// regardless of the child's position or padding.
class _HighlightWrapper extends StatelessWidget {
  const _HighlightWrapper({
    required this.color,
    required this.child,
    double? padding,
  }) : padding = padding ?? 10;

  final Color? color;
  final Widget child;
  final double padding;

  @override
  Widget build(BuildContext context) {
    if (color == null) return child;

    return Stack(
      clipBehavior: Clip.none,
      children: [
        // Full-width highlight background
        Positioned.fill(
          child: IgnorePointer(
            child: _FullWidthHighlight(color: color!, padding: padding),
          ),
        ),
        // The actual component content
        Padding(
          padding: EdgeInsets.symmetric(horizontal: CurrentPlatform.isMobile ? 0.0 : padding),
          child: child,
        ),
      ],
    );
  }
}

class _ScaffoldedComponentWrapper extends StatefulWidget {
  const _ScaffoldedComponentWrapper({
    required this.nodeId,
    required this.linkManager,
    required this.child,
  });

  final String nodeId;
  final Widget child;
  final LeaderLinkManager linkManager;

  @override
  State<_ScaffoldedComponentWrapper> createState() =>
      _ScaffoldedComponentWrapperState();
}

class _ScaffoldedComponentWrapperState
    extends State<_ScaffoldedComponentWrapper> {
  late final LeaderLink _link;

  @override
  Widget build(BuildContext context) {
    return Leader(link: _link, child: widget.child);
  }

  @override
  void dispose() {
    widget.linkManager.releaseLink(widget.nodeId);
    super.dispose();
  }

  @override
  void initState() {
    super.initState();
    _link = widget.linkManager.getLink(widget.nodeId);
    widget.linkManager.retainLink(widget.nodeId);
  }
}
