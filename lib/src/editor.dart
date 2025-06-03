import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
// TODO once `MotionPlugin` properly works, drop this!
import 'package:super_editor/super_editor.dart' show DefaultCaretOverlayBuilder;
import 'package:tempo/flavors.dart';
import 'package:tempo/src/editor/plugins/task/task.dart';

import './editor/super_editor.dart';
import './editor/interaction/interaction.dart';
import './editor/plugins/scaffolding/scaffolding.dart';
import './editor/stylesheet.dart';

export './editor/super_editor.dart';
export './editor/super_text.dart';
export './editor/attributed_text.dart';

SuperEditor createSuperEditor(
  BuildContext context,
  MutableDocument document,
  MutableDocumentComposer composer, {
  required HoverManager hoverManager,
  required LeaderLinkManager linkManager,
}) {
  final editor = Editor(
    requestHandlers: List.from(requestHandlers),
    reactionPipeline: List.from(reactionPipeline),
    historyGroupingPolicy: defaultMergePolicy,
    isHistoryEnabled: true,
    editables: {Editor.documentKey: document, Editor.composerKey: composer},
  );

  editor.context.put(HoverManager.key, hoverManager);

  final superEditor = SuperEditor(
    editor: editor,
    autofocus: true,

    debugPaint:
        F.appFlavor == Flavor.development
            ? DebugPaintConfig(gestures: true, scrolling: false, layout: true)
            : const DebugPaintConfig(),

    plugins: {
      ScaffoldingPlugin(
        hoverManager: hoverManager,
        linkManager: linkManager,
        showDebugPaint: false,
      ),
      // TaskPlugin(),
      MarkdownInlineUpstreamSyntaxPlugin(),
    },

    componentBuilders: [
      ScaffoldedComponentBuilder(
        innerBuilder: TaskComponentBuilder(editor),
        linkManager: linkManager,
      ),
      ScaffoldedComponentBuilder(
        innerBuilder: BlockquoteComponentBuilder(),
        linkManager: linkManager,
      ),
      ScaffoldedComponentBuilder(
        innerBuilder: ParagraphComponentBuilder(),
        linkManager: linkManager,
      ),
      ScaffoldedComponentBuilder(
        innerBuilder: ListItemComponentBuilder(),
        linkManager: linkManager,
      ),
      ScaffoldedComponentBuilder(
        innerBuilder: ImageComponentBuilder(),
        linkManager: linkManager,
      ),
      ScaffoldedComponentBuilder(
        innerBuilder: HorizontalRuleComponentBuilder(),
        linkManager: linkManager,
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

      DefaultCaretOverlayBuilder(),
    ],

    keyboardActions: [...defaultImeKeyboardActions],

    contentTapDelegateFactories: [superEditorLaunchLinkTapHandlerFactory],

    inputSource: TextInputSource.ime,

    stylesheet: buildStylesheet(context),

    // SELECTION BEHAVIOR: How the editor handles focus and selection changes
    selectionPolicies: SuperEditorSelectionPolicies(
      placeCaretAtEndOfDocumentOnGainFocus: false,
      clearSelectionWhenEditorLosesFocus: false,
      clearSelectionWhenImeConnectionCloses: false,
    ),

    // SELECTION COORDINATION: Links for coordinating selection UI across layers
    selectionLayerLinks: SelectionLayerLinks(),
  );

  return superEditor;
}
