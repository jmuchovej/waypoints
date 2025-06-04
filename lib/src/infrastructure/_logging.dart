import 'package:logging/logging.dart';

export 'package:logging/logging.dart'
    show Logger, Level, hierarchicalLoggingEnabled;

import 'package:tempo/src/editor/super_editor.dart';
import 'package:follow_the_leader/follow_the_leader.dart' show FtlLogs;

class LogNames {
  static const devtools = 'devtools';
  static const devtoolsDatabase = '${LogNames.devtools}.database';

  static const plugin = 'plugin';
  static const pluginMotion = '${LogNames.plugin}.motion';
  static const pluginTask = '${LogNames.plugin}.task';
  static const pluginScaffold = '${LogNames.plugin}.scaffold';

  static const layer = 'layer';
  static const layerBlockHighlighter = '${LogNames.layer}.blockHighlighter';

  static const flux = 'flux';
  static const fluxEditor = '${LogNames.flux}.editor';

  static const vox = 'vox';
  static const voxEditor = '${LogNames.vox}.editor';

  static const codex = 'codex';
  static const codexEditor = '${LogNames.codex}.editor';
}

final devtoolsLog = Logger(LogNames.devtools);
final devtoolsDatabaseLog = Logger(LogNames.devtoolsDatabase);

// SuperEditor Plugins
final pluginLog = Logger(LogNames.plugin);
final pluginMotionLog = Logger(LogNames.pluginMotion);
final pluginTaskLog = Logger(LogNames.pluginTask);
final pluginScaffoldLog = Logger(LogNames.pluginScaffold);

// Custom SuperEditor Layers
final layerLog = Logger(LogNames.layer);
final layerBlockHighlighterLog = Logger(LogNames.layerBlockHighlighter);

// Flux
final fluxLog = Logger(LogNames.flux);
final fluxEditorLog = Logger(LogNames.fluxEditor);

// Vox
final voxLog = Logger(LogNames.vox);
final voxEditorLog = Logger(LogNames.voxEditor);

// Codex
final codexLog = Logger(LogNames.codex);
final codexEditorLog = Logger(LogNames.codexEditor);

void initLoggers(Level level, Set<Logger> loggers) {
  hierarchicalLoggingEnabled = true;

  for (final logger in loggers) {
    logger
      ..level = level
      ..onRecord.listen((record) {
        // ignore: avoid_print
        print(record.toString());
      });
  }
}

void deactivateLoggers(Set<Logger> loggers) {
  hierarchicalLoggingEnabled = true;

  for (final logger in loggers) {
    logger.clearListeners();
  }
}

final Set<Logger> tempoLoggers = {
  devtoolsLog,
  devtoolsDatabaseLog,

  // SuperEditor Plugins
  pluginLog,
  pluginMotionLog,
  pluginTaskLog,
  pluginScaffoldLog,

  // Custom SuperEditor Layers
  layerLog,
  layerBlockHighlighterLog,

  // Flux
  fluxLog,
  fluxEditorLog,

  // Vox
  voxLog,
  voxEditorLog,

  // Codex
  codexLog,
  codexEditorLog,
};

final Set<Logger> superEditorLoggers = {
  // Chat
  messagePageElementLog,
  messagePageLayoutLog,
  messagePagePaintLog,
  messageEditorHeightLog,

  // Super Editor
  editorLog,
  editorEditsLog,
  editorPoliciesLog,
  editorScrollingLog,
  editorGesturesLog,
  editorKeyLog,
  editorImeLog,
  editorImeConnectionLog,
  editorImeDeltasLog,
  editorIosFloatingCursorLog,
  editorLayoutLog,
  editorStyleLog,
  editorDocLog,
  editorOpsLog,
  editorTokensLog,
  editorTagsLog,
  editorStableTagsLog,
  editorPatternTagsLog,
  editorActionTagsLog,
  editorSpellingAndGrammarLog,

  // Super Reader
  readerLog,
  readerScrollingLog,
  readerGesturesLog,
  readerKeyLog,
  readerImeLog,
  readerLayoutLog,
  readerStyleLog,
  readerDocLog,
  readerOpsLog,

  textFieldLog,
  scrollingTextFieldLog,
  imeTextFieldLog,
  androidTextFieldLog,
  iosTextFieldLog,

  docGesturesLog,
  infrastructureLog,
  keyboardPanelLog,
  longPressSelectionLog,
  schedulerLog,
  contentLayersLog,
  attributionsLog,
};

// ignore: non_constant_identifier_names
final Set<Logger> FTLLoggers = {
  FtlLogs.leader,
  FtlLogs.follower,
  FtlLogs.link,
  FtlLogs.boundary,
  FtlLogs.widgetBoundary,
};
