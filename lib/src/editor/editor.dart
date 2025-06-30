import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
// TODO once `MotionPlugin` properly works, drop this!
import 'package:super_editor/super_editor.dart' show DefaultCaretOverlayBuilder;

import '../things/project/editor/project_builder.dart';
import '../things/task/editor/task_builder.dart';
import 'nodes.dart';
import 'operations/thing_cursor_handler.dart';
import 'scaffolding.dart';
import 'styling/stylesheet.dart';
import 'super_editor.dart';
import 'operations/task_request_handlers.dart';
import 'operations/panel_request_handlers.dart';
import '../../flavors.dart';
import '../things/task/task_repository.dart';

export 'extensions.dart';
export 'inline.dart';
export 'attributed_text.dart';
export 'nodes.dart';
export 'scaffolding.dart';
export 'super_editor.dart';
export 'super_text.dart';
export 'documents/ordered_set.dart';
export 'operations/task_requests.dart';
export 'operations/panel_requests.dart';

/// Defines how SuperEditor should render in different scrolling contexts
enum SuperEditorRenderMode {
  /// Standard render box with its own scroll controller
  renderBox,
  
  /// Sliver mode for embedding in CustomScrollView
  sliver,
}

/// Creates request handlers that include both default SuperEditor handlers
/// and Tempo-specific task and panel handlers
List<EditRequestHandler> createTempoRequestHandlers(
  TaskRepository taskRepository,
  BuildContext context,
  WidgetRef ref,
) => [
  ...requestHandlers,
  createTaskRequestHandler(
    taskRepository,
  ), // Add our custom task request handler
  createPanelRequestHandler(
    context,
    ref,
  ), // Add our custom panel request handler
];

SuperEditor createSuperEditor(
  BuildContext context,
  MutableDocument document,
  MutableDocumentComposer composer, {
  required HoverManager hoverManager,
  required LeaderLinkManager linkManager,
  required TaskRepository taskRepository,
  required WidgetRef ref,
  ScrollController? scrollController,
  bool? autofocus,
  dynamic panelInfo,
  SuperEditorRenderMode renderMode = SuperEditorRenderMode.renderBox,
}) {
  final editor = Editor(
    requestHandlers: List.from(
      createTempoRequestHandlers(taskRepository, context, ref),
    ),
    reactionPipeline: List.from(reactionPipeline),
    historyGroupingPolicy: defaultMergePolicy,
    isHistoryEnabled: true,
    editables: {Editor.documentKey: document, Editor.composerKey: composer},
  );

  editor.context.put(HoverManager.key, hoverManager);

  // Detect mobile to disable autofocus and prevent keyboard issues
  final screenWidth = MediaQuery.of(context).size.width;
  final isMobile = screenWidth <= 768;

  final flavor = ref.read(flavorProvider);

  final superEditor = SuperEditor(
    editor: editor,
    autofocus: autofocus ?? !isMobile, // Disable autofocus on mobile by default
    // Only use scrollController for renderBox mode
    scrollController: renderMode == SuperEditorRenderMode.renderBox ? scrollController : null,
    // Configure shrinkWrap based on render mode
    shrinkWrap: renderMode == SuperEditorRenderMode.sliver ? false : false,

    debugPaint:
        flavor.isDevelopment
            ? DebugPaintConfig(gestures: true, scrolling: false, layout: true)
            : const DebugPaintConfig(),

    plugins: {
      ScaffoldingPlugin(
        hoverManager: hoverManager,
        linkManager: linkManager,
        showDebugPaint: false,
        scaffoldConfig: DefaultScaffoldConfig(),
      ),
      // TaskPlugin(),
      MarkdownInlineUpstreamSyntaxPlugin(),
    },

    componentBuilders: [
      // Document header is NOT scaffolded - no drag handles, editing, etc.
      const DocumentHeaderComponentBuilder(),

      // Editable text is NOT scaffolded - no drag handles, editing, etc.
      const EditableTextComponentBuilder(),

      ScaffoldedComponentBuilder(
        innerBuilder: TaskComponentBuilder(editor),
        linkManager: linkManager,
        hoverManager: hoverManager,
        composer: composer,
      ),
      ScaffoldedComponentBuilder(
        innerBuilder: ProjectComponentBuilder(),
        linkManager: linkManager,
        hoverManager: hoverManager,
        composer: composer,
      ),
      ScaffoldedComponentBuilder(
        innerBuilder: BlockquoteComponentBuilder(),
        linkManager: linkManager,
        hoverManager: hoverManager,
        composer: composer,
      ),
      ScaffoldedComponentBuilder(
        innerBuilder: ParagraphComponentBuilder(),
        linkManager: linkManager,
        hoverManager: hoverManager,
        composer: composer,
      ),
      ScaffoldedComponentBuilder(
        innerBuilder: ListItemComponentBuilder(),
        linkManager: linkManager,
        hoverManager: hoverManager,
        composer: composer,
      ),
      ScaffoldedComponentBuilder(
        innerBuilder: ImageComponentBuilder(),
        linkManager: linkManager,
        hoverManager: hoverManager,
        composer: composer,
      ),
      ScaffoldedComponentBuilder(
        innerBuilder: HorizontalRuleComponentBuilder(),
        linkManager: linkManager,
        hoverManager: hoverManager,
        composer: composer,
      ),
    ],

    documentOverlayBuilders: [
      // PLATFORM-SPECIFIC UI: iOS text selection and toolbar support
      if (defaultTargetPlatform == TargetPlatform.iOS) ...[
        SuperEditorIosToolbarFocalPointDocumentLayerBuilder(),
        SuperEditorIosHandlesDocumentLayerBuilder(),
      ],

      // PLATFORM-SPECIFIC UI: Android text selection and toolbar support
      if (defaultTargetPlatform == TargetPlatform.android) ...[
        SuperEditorAndroidToolbarFocalPointDocumentLayerBuilder(),
        SuperEditorAndroidHandlesDocumentLayerBuilder(),
      ],

      DefaultCaretOverlayBuilder(
        caretStyle: const CaretStyle().copyWith(
          color: ColorScheme.of(context).primary,
        ),
      ),
    ],

    keyboardActions: [...defaultImeKeyboardActions],

    contentTapDelegateFactories: [
      superEditorLaunchLinkTapHandlerFactory,
      // Custom cursor handler for ThingNodes (tasks, projects, etc.)
      (editContext) => ThingCursorHandler(
        editContext: editContext,
        context: context,
        ref: ref,
        panelInfo: panelInfo,
      ),
    ],

    inputSource: TextInputSource.ime,

    stylesheet: buildStylesheet(context),

    // SELECTION BEHAVIOR: How the editor handles focus and selection changes
    selectionPolicies: SuperEditorSelectionPolicies(
      placeCaretAtEndOfDocumentOnGainFocus: false,
      openKeyboardWhenTappingExistingSelection:
          isMobile ? false : true, // Aggressive keyboard control on mobile
      clearSelectionWhenEditorLosesFocus:
          isMobile
              ? true
              : false, // Clear selection on mobile to prevent keyboard
      clearSelectionWhenImeConnectionCloses:
          isMobile ? true : false, // Clear when keyboard closes on mobile
    ),

    // SELECTION COORDINATION: Links for coordinating selection UI across layers
    selectionLayerLinks: SelectionLayerLinks(),
  );

  return superEditor;
}

extension DocumentChanges on MutableDocument {
  void deleteAllButHeader() {
    final header = firstOrNull;
    if (header == null) return;

    final nodes = toList();
    for (final node in nodes) {
      if (node.id == header.id) continue;

      deleteNode(node.id);
    }
  }
}
