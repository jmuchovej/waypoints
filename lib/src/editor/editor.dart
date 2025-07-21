import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
// TODO once `MotionPlugin` properly works, drop this!
import 'package:super_editor/super_editor.dart' show DefaultCaretOverlayBuilder;

import '../../flavors.dart';
import '../styling/super_editor.dart';
import '../styling/tempo_stylesheet.dart';
import '../things/task/task_repository.dart';
import 'listeners/autosave_listener.dart';
import 'nodes/project_builder.dart';
import 'nodes/task_builder.dart';
import 'operations/task_status.dart';
import 'operations/thing_cursor_handler.dart';
import 'scaffolding.dart';
import 'super_editor.dart';

export 'attributed_text.dart';
export 'documents/ordered_set.dart';
export 'extensions.dart';
export 'inline.dart';
export 'operations/task_status.dart';
export 'scaffolding.dart';
export 'super_editor.dart';
export 'super_text.dart';

SuperEditor createSuperEditor(
  BuildContext context,
  MutableDocument document,
  MutableDocumentComposer composer, {
  required HoverManager hoverManager,
  required LeaderLinkManager linkManager,
  required TaskRepository taskRepository,
  required WidgetRef ref,
  bool? autofocus,
  dynamic panelInfo,
}) {
  // TODO: Fix FollowerLayer null check error when document is empty
  // If document is empty, inject a placeholder node with hint text like:
  // "Start typing to add content..." or "Click here to add your first note"
  // This prevents SuperEditor + FollowerLayer crashes on empty documents
  // and provides better UX guidance for users.
  final editor = Editor(
    requestHandlers: List.from(
      createTempoRequestHandlers(taskRepository, context, ref),
    ),
    reactionPipeline: List.from(reactionPipeline),
    historyGroupingPolicy: defaultMergePolicy,
    isHistoryEnabled: true,
    editables: {Editor.documentKey: document, Editor.composerKey: composer},
    listeners: [AutoSaveListener(ref)],
  );

  editor.context.put(HoverManager.key, hoverManager);

  // Detect mobile to disable autofocus and prevent keyboard issues
  final stylesheet = ref.read(tempoStylesheetProvider);
  final isMobile = stylesheet.isMobile;

  final flavor = ref.read(flavorProvider);

  final superEditor = SuperEditor(
    editor: editor,
    autofocus: autofocus ?? !isMobile, // Disable autofocus on mobile by default
    scrollController: null,
    shrinkWrap: false,

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

    stylesheet: buildStylesheet(ref),

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

/// Creates request handlers that include both default SuperEditor handlers
/// and Tempo-specific task and panel handlers
List<EditRequestHandler> createTempoRequestHandlers(
  TaskRepository taskRepository,
  BuildContext context,
  WidgetRef ref,
) => [...requestHandlers, changeTaskStatusHandler];

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
