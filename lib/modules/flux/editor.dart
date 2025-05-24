import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
// import 'package:tempo/core/editor_appflowy/_keybinds.dart';
// import 'package:tempo/core/editor_appflowy/_shortcuts.dart';
// import 'package:tempo/core/editor_appflowy/appflowy_editor.dart';
import 'package:tempo/core/editor/stylesheet.dart';
import 'package:tempo/core/editor/super_editor.dart';
import 'package:tempo/flavors.dart';

export 'package:tempo/core/editor/super_editor.dart';
// export 'package:tempo/core/editor_appflowy/appflowy_editor.dart';

SuperEditor createSuperEditor(
  BuildContext context,
  MutableDocument document,
  MutableDocumentComposer composer,
  Set<SuperEditorPlugin> plugins,
) {
  // final editor = createDefaultDocumentEditor(
  //   document: document,
  //   composer: composer,
  //   isHistoryEnabled: true,
  // );

  final editor = Editor(
    requestHandlers: List.from(requestHandlers),
    reactionPipeline: List.from(reactionPipeline),
    historyGroupingPolicy: defaultMergePolicy,
    isHistoryEnabled: true,
    editables: {Editor.documentKey: document, Editor.composerKey: composer},
  );

  final superEditor = SuperEditor(
    editor: editor,
    autofocus: true,
    debugPaint:
        F.appFlavor == Flavor.development
            ? DebugPaintConfig(gestures: true, scrolling: false, layout: true)
            : const DebugPaintConfig(),
    plugins: plugins,
    contentTapDelegateFactories: null,
    // contentTapDelegateFactories: const [
    //   superEditorAddEmptyNodeTapHandlerFactory<TaskNode>,
    // ],
    componentBuilders: componentBuilders,
    // TODO figure out what's needed to provide these ourselves...
    keyboardActions: const [],
    stylesheet: buildStylesheet(context),
    /// Default list of document overlays that are displayed on top of the document
    /// layout in a [SuperEditor].
    documentOverlayBuilders: [
      // Displays caret for typical desktop use-cases.
      CaretOverlayBuilder(
        caretStyle: const CaretStyle().copyWith(
          color: ColorScheme.of(context).primary,
        ),
      ),
      if (defaultTargetPlatform == TargetPlatform.iOS) ...[
        // Adds a Leader around the document selection at a focal point for the
        // iOS floating toolbar.
        SuperEditorIosToolbarFocalPointDocumentLayerBuilder(),
        // Displays caret and drag handles, specifically for iOS.
        SuperEditorIosHandlesDocumentLayerBuilder(),
      ],

      if (defaultTargetPlatform == TargetPlatform.android) ...[
        // Adds a Leader around the document selection at a focal point for the
        // Android floating toolbar.
        SuperEditorAndroidToolbarFocalPointDocumentLayerBuilder(),
        // Displays caret and drag handles, specifically for Android.
        SuperEditorAndroidHandlesDocumentLayerBuilder(),
      ],
    ],
    // inputSource: TextInputSource.ime,
    selectionPolicies: SuperEditorSelectionPolicies(
      placeCaretAtEndOfDocumentOnGainFocus: false,
      clearSelectionWhenEditorLosesFocus: false,
      clearSelectionWhenImeConnectionCloses: false,
    ),
    selectionLayerLinks: SelectionLayerLinks(),
  );

  return superEditor;
}

// AppFlowyEditor createAppFlowyEditor(
//   EditorState state,
//   EditorStyle style,
// ) {
//   final appflowyEditor = AppFlowyEditor(
//     editorState: state,
//     blockComponentBuilders: componentBuilders,
//     commandShortcutEvents: keyboundEvents,
//     characterShortcutEvents: shortcutEvents,
//     editorStyle: style,
//   );
//   return appflowyEditor;
// }
