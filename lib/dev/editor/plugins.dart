import 'package:tempo/src/editor/super_editor.dart';

class DevToolsPlugin implements SuperEditorPlugin {
  @override
  // TODO: implement appendedStylePhases
  List<SingleColumnLayoutStylePhase> get appendedStylePhases => const [];

  @override
  void attach(Editor editor) {}

  @override
  // TODO: implement componentBuilders
  List<ComponentBuilder> get componentBuilders => const [];

  @override
  // TODO: implement contentTapHandlers
  List<ContentTapDelegate> get contentTapHandlers => const [];

  @override
  void detach(Editor editor) {
    // TODO: implement detach
  }

  @override
  // TODO: implement documentOverlayBuilders
  List<SuperEditorLayerBuilder> get documentOverlayBuilders => const [];

  @override
  // TODO: implement documentUnderlayBuilders
  List<SuperEditorLayerBuilder> get documentUnderlayBuilders => const [];

  @override
  // TODO: implement keyboardActions
  List<DocumentKeyboardAction> get keyboardActions => const [];
}
