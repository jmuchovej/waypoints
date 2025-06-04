import 'package:flutter/material.dart';
import 'package:tempo/src/editor/layers/block_highlighter.dart';
import 'package:tempo/src/editor/super_editor.dart';
import 'package:tempo/src/infrastructure/platform.dart';
import 'package:tempo/src/editor/nodes/_thing.dart';

import '_keybinds.dart';
import '_layers.dart';
import '_caret.dart';

enum MotionMode { normal, insert, mobile }

enum MotionDirection { N, E, W, S }

// Global reference to the current motion plugin for spacebar handling
MotionPlugin? _currentMotionPlugin;

final ValueNotifier<MotionMode> motionMode = ValueNotifier(MotionMode.normal);

class Motion extends ValueNotifier<MotionMode> implements Editable {
  static const key = "motion";
  Motion(super.value);

  void activateNormal() => value = MotionMode.normal;
  void activateInsert() => value = MotionMode.insert;
  void activateMobile() => value = MotionMode.mobile;

  @override
  void onTransactionEnd(List<EditEvent> edits) {}

  @override
  void onTransactionStart() {}

  @override
  void reset() {}
}

extension MotionString on MotionMode {
  String toDisplayString() {
    return toString().split(".").last.toUpperCase();
  }
}

extension MotionColor on MotionMode {
  Color get color {
    switch (this) {
      case MotionMode.normal:
        return Colors.green;
      case MotionMode.insert:
        return Colors.blue;
      case MotionMode.mobile:
        return Colors.purple;
    }
  }
}

class MotionPlugin extends SuperEditorPlugin {
  MotionPlugin({
    Color? normalModeColor,
    Color? insertModeColor,

    /// Optional navigation function for Things. If not provided,
    /// the plugin will try to use GoRouter from context.
    void Function(String uuid)? onNavigateToThing,
  }) : normalModeColor = normalModeColor ?? Colors.deepPurple.shade300,
       insertModeColor = insertModeColor ?? Colors.transparent,
       _onNavigateToThing = onNavigateToThing {
    _contentTapDelegate = MotionContentTapDelegate(
      motionMode,
      _onNavigateToThing,
    );
  }

  final Color normalModeColor;
  final Color insertModeColor;
  final void Function(String uuid)? _onNavigateToThing;

  late final Motion motionMode = Motion(
    CurrentPlatform.isMobile ? MotionMode.mobile : MotionMode.normal,
  );

  late final MotionContentTapDelegate _contentTapDelegate;

  @override
  void attach(Editor editor) {
    super.attach(editor);
    editor.context.put(Motion.key, motionMode);
    _contentTapDelegate.editor = editor;
    _currentMotionPlugin = this;
    if (CurrentPlatform.isMobile) {
      motionMode.activateMobile();
    }
    motionMode.addListener(() {
      switch (motionMode.value) {
        case MotionMode.normal:
          editor.execute([
            const ChangeInteractionModeRequest(isInteractionModeDesired: true),
          ]);
          break;
        case MotionMode.insert:
          editor.execute([
            const ChangeInteractionModeRequest(isInteractionModeDesired: false),
          ]);
          break;
        default:
          return;
      }
    });
  }

  @override
  void detach(Editor editor) {
    if (CurrentPlatform.isMobile) {
      motionMode.activateMobile();
    }
    _contentTapDelegate.editor = null;
    editor.context.remove(Motion.key);
    _currentMotionPlugin = null;
    super.detach(editor);
  }

  /// Static access to the current plugin's navigation function for keyboard handlers
  static void Function(String uuid)? get currentNavigateToThing =>
      _currentMotionPlugin?._onNavigateToThing;

  @override
  List<DocumentKeyboardAction> get keyboardActions =>
      CurrentPlatform.isMobile
          ? const []
          : [
            escToReturnToNormalMode,
            iToEnterInsertMode,
            normalModeMoveN,
            normalModeMoveS,
            normalModeMoveE,
            normalModeMoveW,
            spacebarToEditTask,
            blockCharacterInsertionInNormalMode,
          ];

  @override
  List<SuperEditorLayerBuilder> get documentUnderlayBuilders => [
    // MotionSelectionHighlightBuilder(
    //   normalModeColor: normalModeColor,
    // ),
    BlockHighlightBuilder(),
  ];
  @override
  List<SuperEditorLayerBuilder> get documentOverlayBuilders => [
    MotionIndicatorOverlayBuilder(mode: motionMode),
    MotionAwareCaretOverlayBuilder(
      caretStyle: CaretStyle(width: 2, color: normalModeColor),
      displayOnAllPlatforms: true,
    ),
  ];

  @override
  List<ContentTapDelegate> get contentTapHandlers => [
    // _contentTapDelegate,
  ];
}

class MotionContentTapDelegate extends ContentTapDelegate {
  MotionContentTapDelegate(this.motionMode, this._onNavigateToThing);

  final Motion motionMode;
  final void Function(String uuid)? _onNavigateToThing;
  Editor? editor;

  @override
  MouseCursor? mouseCursorForContentHover(DocumentPosition hoverPosition) {
    if (editor == null || motionMode.value != MotionMode.normal) return null;

    // VISUAL FEEDBACK: Show pointer cursor when hovering over Thing nodes
    // in normal mode to indicate they're clickable for navigation
    final node = editor!.document.getNodeById(hoverPosition.nodeId);
    if (node is ThingNode) {
      return SystemMouseCursors.click;
    }
    return null;
  }

  @override
  TapHandlingInstruction onTap(DocumentTapDetails details) {
    // In normal mode, handle clicks on Thing nodes
    if (motionMode.value == MotionMode.normal) {
      // IMPORTANT FIX: Use getDocumentPositionAtOffset (not getDocumentPositionNearestToOffset)
      // for precise hit testing. This prevents clicks in empty space below the last
      // node from incorrectly triggering that node.
      //
      // - getDocumentPositionAtOffset: only returns a position if you click directly on content
      // - getDocumentPositionNearestToOffset: returns the nearest node even if you click outside it
      final tapPosition = details.documentLayout.getDocumentPositionAtOffset(
        details.layoutOffset,
      );

      if (tapPosition != null && editor != null) {
        final node = editor!.document.getNodeById(tapPosition.nodeId);

        // THING NAVIGATION: If we clicked on any Thing node (Task, Project, etc.),
        // navigate directly to its dedicated page at /thing/:uuid
        // This provides a consistent navigation pattern for all Things in normal mode
        if (node is ThingNode && _onNavigateToThing != null) {
          _onNavigateToThing(node.thingUuid);
          return TapHandlingInstruction.halt;
        }
      }

      // Prevent default selection/caret placement in normal mode
      return TapHandlingInstruction.halt;
    }
    // Let default behavior proceed in insert mode
    return TapHandlingInstruction.continueHandling;
  }

  @override
  TapHandlingInstruction onDoubleTap(DocumentTapDetails details) {
    return TapHandlingInstruction.continueHandling;
  }

  @override
  TapHandlingInstruction onPanStart(DocumentTapDetails details) {
    return TapHandlingInstruction.continueHandling;
  }

  @override
  TapHandlingInstruction onPanUpdate(DocumentTapDetails details) {
    return TapHandlingInstruction.continueHandling;
  }

  @override
  TapHandlingInstruction onPanEnd(DocumentTapDetails details) {
    return TapHandlingInstruction.continueHandling;
  }
}
