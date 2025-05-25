import 'package:flutter/material.dart';
import 'package:tempo/core/editor/super_editor.dart';
import 'package:tempo/infrastructure/platform.dart';

import '_keybinds.dart';
import '_layers.dart';

enum MotionMode { normal, insert, mobile }

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

enum MotionDirection { N, E, W, S }

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

class MotionPlugin extends SuperEditorPlugin {
  MotionPlugin({Color? normalModeColor, Color? insertModeColor})
    : normalModeColor = normalModeColor ?? Colors.deepPurple.shade300,
      insertModeColor = insertModeColor ?? Colors.transparent;

  final Color normalModeColor;
  final Color insertModeColor;

  final Motion motionMode = Motion(
    CurrentPlatform.isMobile ? MotionMode.mobile : MotionMode.normal,
  );

  @override
  void attach(Editor editor) {
    super.attach(editor);
    editor.context.put(Motion.key, motionMode);
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
    editor.context.remove(Motion.key);
    super.detach(editor);
  }

  @override
  List<DocumentKeyboardAction> get keyboardActions =>
      CurrentPlatform.isMobile
          ? const []
          : [
            escToReturnToNormalMode,
            iToEnterInsertMode,
            normalModeMoveN,
            normalModeMoveS,
            blockCharacterInsertionInNormalMode,
          ];

  @override
  List<SuperEditorLayerBuilder> get documentUnderlayBuilders => [
    NodeSelectionUnderlayBuilder(
      normalModeColor: normalModeColor,
      insertModeColor: insertModeColor,
    ),
  ];
  @override
  List<SuperEditorLayerBuilder> get documentOverlayBuilders => [
    MotionIndicatorOverlayBuilder(mode: motionMode),
  ];
}

