import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:super_editor/super_editor.dart';

import '../../router.dart';
import 'task_builder.dart';
import '../../things/project/project.entity.dart';
import '../../objectbox/repositories/task_repository.dart';
import '../../objectbox/repositories/project_repository.dart';
import '../../infrastructure/logging.dart';
import '../operations/thing_cursor_handler.dart';
import '../operations/thing_node_position.dart';
import 'project_builder.dart';

/// The visual component for displaying a project in the editor
class ProjectComponent extends ConsumerStatefulWidget {
  const ProjectComponent({super.key, required this.viewModel});

  final ProjectComponentViewModel viewModel;

  @override
  ConsumerState<ProjectComponent> createState() => _ProjectComponentState();
}

class _ProjectComponentState extends ConsumerState<ProjectComponent>
    with
        ProxyDocumentComponent<ProjectComponent>,
        ProxyTextComposable,
        ThingCursorProvider {
  bool _isHovered = false;
  int? _taskCount;

  final _textKey = GlobalKey();

  // Debug flag - set to true to see detailed gesture logging
  static const bool _debugGestures = true;

  ProjectEntity get project => widget.viewModel.project;

  @override
  GlobalKey<State<StatefulWidget>> get childDocumentComponentKey => _textKey;

  @override
  TextComposable get childTextComposable =>
      childDocumentComponentKey.currentState as TextComposable;

  @override
  void initState() {
    super.initState();
    _loadTaskCount();
  }

  void _loadTaskCount() async {
    try {
      final taskRepo = ref.read(taskRepositoryProvider);
      final tasks = taskRepo.sameTypedChildren(project.uuid);
      setState(() {
        _taskCount = tasks.length;
      });
    } catch (e) {
      waypointsProjectEditorLog.warning(
        'Failed to load task count for project: $e',
      );
      setState(() {
        _taskCount = 0;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final textSize = TextTheme.of(context).bodyLarge!.fontSize!;

    return Directionality(
      textDirection: widget.viewModel.textDirection,
      child: Padding(
        padding: EdgeInsets.symmetric(vertical: textSize / 2),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SizedBox(
              width: tempoTaskIndentCalculator(
                widget.viewModel.textStyleBuilder({}),
                widget.viewModel.indent,
              ),
            ),
            Column(children: [Icon(Icons.folder_rounded)]),
            const SizedBox(width: 8),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  TextComponent(
                    key: _textKey,
                    text: widget.viewModel.text,
                    textDirection: widget.viewModel.textDirection,
                    textAlign: widget.viewModel.textAlignment,
                    textStyleBuilder: _computeStyles,
                    inlineWidgetBuilders: widget.viewModel.inlineWidgetBuilders,
                    textSelection: widget.viewModel.selection,
                    selectionColor: widget.viewModel.selectionColor,
                    highlightWhenEmpty: widget.viewModel.highlightWhenEmpty,
                    underlines: widget.viewModel.createUnderlines(),
                    showDebugPaint: true,
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );

    // return MouseRegion(
    //   key: widget.componentKey,
    //   onEnter: (_) => setState(() => _isHovered = true),
    //   onExit: (_) => setState(() => _isHovered = false),
    //   child: GestureDetector(
    //     onTap: () => _openProject(context),
    //     child: AnimatedContainer(
    //       duration: const Duration(milliseconds: 150),
    //       margin: const EdgeInsets.symmetric(vertical: 4, horizontal: 8),
    //       padding: const EdgeInsets.all(16),
    //       decoration: BoxDecoration(
    //         color:
    //             isSelected
    //                 ? theme.colorScheme.primaryContainer.withValues(alpha: 0.3)
    //                 : _isHovered
    //                 ? theme.colorScheme.surfaceContainerHighest
    //                 : theme.colorScheme.surfaceContainer,
    //         borderRadius: BorderRadius.circular(12),
    //         border: Border.all(
    //           color:
    //               isSelected
    //                   ? theme.colorScheme.primary
    //                   : _isHovered
    //                   ? theme.colorScheme.outline.withValues(alpha: 0.3)
    //                   : Colors.transparent,
    //           width: isSelected ? 2 : 1,
    //         ),
    //         boxShadow:
    //             _isHovered
    //                 ? [
    //                   BoxShadow(
    //                     color: theme.shadowColor.withValues(alpha: 0.1),
    //                     blurRadius: 8,
    //                     offset: const Offset(0, 2),
    //                   ),
    //                 ]
    //                 : null,
    //       ),
    //       child: Column(
    //         crossAxisAlignment: CrossAxisAlignment.start,
    //         children: [
    //           // Header row with title and actions
    //           Row(
    //             children: [
    //               // Project icon
    //               Container(
    //                 padding: const EdgeInsets.all(8),
    //                 decoration: BoxDecoration(
    //                   color: theme.colorScheme.primaryContainer,
    //                   borderRadius: BorderRadius.circular(8),
    //                 ),
    //                 child: Icon(
    //                   Icons.folder,
    //                   size: 20,
    //                   color: theme.colorScheme.onPrimaryContainer,
    //                 ),
    //               ),
    //               const SizedBox(width: 12),

    //               // Project title
    //               Expanded(
    //                 child: Text(
    //                   widget.project.title.toPlainText(),
    //                   style: theme.textTheme.titleMedium?.copyWith(
    //                     fontWeight: FontWeight.w600,
    //                   ),
    //                   maxLines: 2,
    //                   overflow: TextOverflow.ellipsis,
    //                 ),
    //               ),

    //               // Action buttons (visible on hover)
    //               AnimatedOpacity(
    //                 duration: const Duration(milliseconds: 150),
    //                 opacity: _isHovered ? 1.0 : 0.0,
    //                 child: Row(
    //                   mainAxisSize: MainAxisSize.min,
    //                   children: [
    //                     IconButton(
    //                       icon: const Icon(Icons.edit, size: 16),
    //                       onPressed: () => _editProject(context),
    //                       tooltip: 'Edit Project',
    //                       constraints: const BoxConstraints(
    //                         minWidth: 32,
    //                         minHeight: 32,
    //                       ),
    //                     ),
    //                     IconButton(
    //                       icon: const Icon(Icons.more_vert, size: 16),
    //                       onPressed: () => _showProjectMenu(context),
    //                       tooltip: 'More Options',
    //                       constraints: const BoxConstraints(
    //                         minWidth: 32,
    //                         minHeight: 32,
    //                       ),
    //                     ),
    //                   ],
    //                 ),
    //               ),
    //             ],
    //           ),

    //           const SizedBox(height: 12),

    //           // Project metadata
    //           Row(
    //             children: [
    //               // Task count
    //               if (_taskCount != null) ...[
    //                 Container(
    //                   padding: const EdgeInsets.symmetric(
    //                     horizontal: 8,
    //                     vertical: 4,
    //                   ),
    //                   decoration: BoxDecoration(
    //                     color: theme.colorScheme.surfaceContainerHighest,
    //                     borderRadius: BorderRadius.circular(12),
    //                   ),
    //                   child: Row(
    //                     mainAxisSize: MainAxisSize.min,
    //                     children: [
    //                       Icon(
    //                         Icons.task_alt,
    //                         size: 12,
    //                         color: theme.colorScheme.onSurfaceVariant,
    //                       ),
    //                       const SizedBox(width: 4),
    //                       Text(
    //                         '$_taskCount ${_taskCount == 1 ? 'task' : 'tasks'}',
    //                         style: theme.textTheme.bodySmall?.copyWith(
    //                           color: theme.colorScheme.onSurfaceVariant,
    //                         ),
    //                       ),
    //                     ],
    //                   ),
    //                 ),
    //                 const SizedBox(width: 8),
    //               ],

    //               // Modified date
    //               // Text(
    //               //   Jiffy.now().diff(Jiffy.parseFromDateTime(widget.project.modifyDate),
    //               //   style: theme.textTheme.bodySmall?.copyWith(
    //               //     color: theme.colorScheme.onSurfaceVariant,
    //               //   ),
    //               // ),
    //               const Spacer(),

    //               // Status indicators
    //               if (widget.project.archived)
    //                 Container(
    //                   padding: const EdgeInsets.symmetric(
    //                     horizontal: 6,
    //                     vertical: 2,
    //                   ),
    //                   decoration: BoxDecoration(
    //                     color: theme.colorScheme.errorContainer,
    //                     borderRadius: BorderRadius.circular(8),
    //                   ),
    //                   child: Text(
    //                     'Archived',
    //                     style: theme.textTheme.labelSmall?.copyWith(
    //                       color: theme.colorScheme.onErrorContainer,
    //                     ),
    //                   ),
    //                 ),
    //             ],
    //           ),

    //           // Project description (if available)
    //           // if (widget.project.notes != null &&
    //           //     widget.project.notes!.toPlainText().trim().isNotEmpty) ...[
    //           //   const SizedBox(height: 8),
    //           //   Text(
    //           //     widget.project.notes!.toPlainText(),
    //           //     style: theme.textTheme.bodySmall?.copyWith(
    //           //       color: theme.colorScheme.onSurfaceVariant,
    //           //     ),
    //           //     maxLines: 2,
    //           //     overflow: TextOverflow.ellipsis,
    //           //   ),
    //           // ],
    //         ],
    //       ),
    //     ),
    //   ),
    // );
  }

  @override
  NodePosition? getPositionAtOffset(Offset localOffset) {
    // Determine which cursor zone this offset is in
    final cursorZone = getCursorZoneForOffset(localOffset);

    if (cursorZone == ThingCursorZone.textEditable) {
      // For text areas, return the actual text position
      return _getTextComponentPositionAtOffset(localOffset);
    } else {
      // For non-text areas, return a ThingNodePosition with zone metadata
      // Use -1 offset as convention for non-text positions
      return ThingNodePosition(offset: -1, zone: cursorZone);
    }
  }

  bool _isClickInTextArea(Offset localOffset) {
    try {
      // Get the text component that's nested inside our Row layout
      final textComponentBox =
          childDocumentComponentKey.currentContext?.findRenderObject()
              as RenderBox?;
      if (textComponentBox == null) {
        waypointsThingTapDelegateLog.finest(
          '🎯 _isClickInTextArea: No text component box found',
        );
        return false;
      }

      // Get our project component's render box
      final projectComponentBox = context.findRenderObject() as RenderBox?;
      if (projectComponentBox == null) {
        waypointsThingTapDelegateLog.finest(
          '🎯 _isClickInTextArea: No project component box found',
        );
        return false;
      }

      // Convert the local offset to the text component's coordinate space
      final textOffset = projectComponentBox.globalToLocal(
        textComponentBox.localToGlobal(Offset.zero),
      );

      // Check if the click is within the text component bounds
      final textBounds = Rect.fromLTWH(
        textOffset.dx,
        textOffset.dy,
        textComponentBox.size.width,
        textComponentBox.size.height,
      );

      final isInTextArea = textBounds.contains(localOffset);

      if (_debugGestures) {
        waypointsProjectEditorLog.finest(
          '🎯 _isClickInTextArea: click=$localOffset, textBounds=$textBounds, result=$isInTextArea',
        );
      }

      return isInTextArea;
    } catch (e) {
      if (_debugGestures) {
        waypointsProjectEditorLog.warning(
          '🎯 _isClickInTextArea: Error calculating text area: $e',
        );
      }
      // Default to false (not in text area) on any error
      return false;
    }
  }

  NodePosition? _getTextComponentPositionAtOffset(Offset localOffset) {
    // Get the text component that's nested inside our Row layout
    final textComponentBox =
        childDocumentComponentKey.currentContext?.findRenderObject()
            as RenderBox?;
    if (textComponentBox == null) {
      // Fallback to beginning if we can't find the text component
      return const TextNodePosition(offset: 0);
    }

    // Get our project component's render box
    final projectComponentBox = context.findRenderObject() as RenderBox?;
    if (projectComponentBox == null) {
      return const TextNodePosition(offset: 0);
    }

    try {
      // Convert the click position from project component coordinates to text component coordinates
      // This accounts for the folder icon that appears before the text in the Row
      final globalClickPosition = projectComponentBox.localToGlobal(
        localOffset,
      );
      final textLocalOffset = textComponentBox.globalToLocal(
        globalClickPosition,
      );

      // Now get the position within the text using the adjusted coordinates
      final textComponent =
          childDocumentComponentKey.currentState as TextComponentState?;
      final position = textComponent?.getPositionAtOffset(textLocalOffset);

      // Return the calculated position, or beginning if calculation failed
      return position ?? const TextNodePosition(offset: 0);
    } catch (e) {
      // If coordinate conversion fails, place cursor at beginning
      return const TextNodePosition(offset: 0);
    }
  }

  /// Computes the [TextStyle] for this task's inner [TextComponent].
  TextStyle _computeStyles(Set<Attribution> attributions) {
    // Show a strikethrough across the entire task if it's complete.
    final style = widget.viewModel.textStyleBuilder(attributions);
    return style;
  }

  void _openProject(BuildContext context) {
    context.goProject(project, ref);
  }

  void _editProject(BuildContext context) {
    // TODO: Navigate to project editor
    // context.router.push(ProjectEditorRoute(projectId: widget.project.uuid));
  }

  void _showProjectMenu(BuildContext context) {
    final theme = Theme.of(context);

    showModalBottomSheet(
      context: context,
      builder:
          (context) => Container(
            padding: const EdgeInsets.all(16),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  project.title.toPlainText(),
                  style: theme.textTheme.titleMedium,
                ),
                const SizedBox(height: 16),
                ListTile(
                  leading: const Icon(Icons.edit),
                  title: const Text('Edit Project'),
                  onTap: () {
                    Navigator.pop(context);
                    _editProject(context);
                  },
                ),
                ListTile(
                  leading: Icon(
                    project.archived ? Icons.unarchive : Icons.archive,
                  ),
                  title: Text(project.archived ? 'Unarchive' : 'Archive'),
                  onTap: () {
                    Navigator.pop(context);
                    _toggleArchive();
                  },
                ),
                const Divider(),
                ListTile(
                  leading: const Icon(Icons.delete, color: Colors.red),
                  title: const Text(
                    'Delete Project',
                    style: TextStyle(color: Colors.red),
                  ),
                  onTap: () {
                    Navigator.pop(context);
                    _confirmDelete(context);
                  },
                ),
              ],
            ),
          ),
    );
  }

  void _toggleArchive() {
    final projectRepo = ref.read(projectRepositoryProvider);
    projectRepo.archive(project.uuid);
    _loadTaskCount(); // Refresh display
  }

  void _confirmDelete(BuildContext context) {
    showDialog(
      context: context,
      builder:
          (context) => AlertDialog(
            title: const Text('Delete Project'),
            content: Text(
              'Are you sure you want to delete "${project.title.toPlainText()}"?\n\n'
              'Tasks in this project will not be deleted but will no longer be associated with the project.',
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context),
                child: const Text('Cancel'),
              ),
              TextButton(
                style: TextButton.styleFrom(foregroundColor: Colors.red),
                onPressed: () {
                  Navigator.pop(context);
                  _deleteProject();
                },
                child: const Text('Delete'),
              ),
            ],
          ),
    );
  }

  void _deleteProject() {
    final projectRepo = ref.read(projectRepositoryProvider);
    projectRepo.remove(project.id);
  }

  @override
  ThingCursorZone getCursorZoneForOffset(Offset localOffset) {
    try {
      // Folder icon area (left side) - interactive/clickable for navigation
      if (localOffset.dx < 40) {
        waypointsThingTapDelegateLog.finest(
          '🎯 Cursor zone: folder icon area (clickable)',
        );
        return ThingCursorZone.clickable;
      }

      // Check if we're in the text area
      if (_isClickInTextArea(localOffset)) {
        waypointsThingTapDelegateLog.finest(
          '🎯 Cursor zone: text area (editable)',
        );
        return ThingCursorZone.textEditable;
      }

      // Task count/metadata area (right side) - could be interactive if we add chips
      final componentBox = context.findRenderObject() as RenderBox?;
      if (componentBox != null) {
        final rightZoneStart = componentBox.size.width * 0.7; // Approximate
        if (localOffset.dx > rightZoneStart) {
          waypointsThingTapDelegateLog.finest(
            '🎯 Cursor zone: metadata area (clickable)',
          );
          return ThingCursorZone.clickable;
        }
      }

      // Everything else is clickable (for navigation)
      waypointsThingTapDelegateLog.finest(
        '🎯 Cursor zone: general area (clickable)',
      );
      return ThingCursorZone.clickable;
    } catch (e) {
      waypointsThingTapDelegateLog.warning(
        '🎯 Error determining cursor zone: $e',
      );
      // Default to clickable on any error
      return ThingCursorZone.clickable;
    }
  }
}
