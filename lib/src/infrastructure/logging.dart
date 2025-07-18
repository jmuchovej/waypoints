// ignore_for_file: non_constant_identifier_names

import 'dart:developer' as dev;

import 'package:flutter/foundation.dart';
import 'package:logging/logging.dart';

export 'package:logging/logging.dart'
    show Logger, Level, hierarchicalLoggingEnabled;

import 'package:follow_the_leader/follow_the_leader.dart' show FtlLogs;
import 'package:super_editor/super_editor.dart' as SE hide Logger;

class LogNames {
  static const tempo = 'tempo';

  static const editor = '${LogNames.tempo}.editor';
  static const taskEditor = '${LogNames.editor}.task';
  static const projectEditor = '${LogNames.editor}.project';

  // SuperEditor plugins
  static const motionPlugin = '${LogNames.editor}.motion';
  static const scaffoldingPlugin = '${LogNames.editor}.scaffolding';

  // Custom SuperEditor layers
  static const editorLayer = '${LogNames.editor}.layer';
  static const blockHighlighterLayer =
      '${LogNames.editor}.layer.blockHighlighter';
  static const editorTapDelegate = '${LogNames.editor}.tapDelegate';
  static const thingTapDelegate =
      '${LogNames.editorTapDelegate}.tapDelegate.thing';

  // Styling
  static const style = '${LogNames.tempo}.style';

  // Paneling
  static const ui = '${LogNames.tempo}.ui';
  static const uiPanel = '${LogNames.ui}.panel';

  // Routing
  static const route = '${LogNames.tempo}.route';
  static const taskRoute = '${LogNames.route}.task';
  static const thingRoute = '${LogNames.route}.thing';

  // Database subsystem
  static const db = '${LogNames.tempo}.db';
  static const dbSeed = '${LogNames.db}.seed';
  static const dbQuery = '${LogNames.db}.query';
  static const dbSchema = '${LogNames.db}.schema';

  // Development tools
  static const devtools = '${LogNames.tempo}.devtools';

  static const state = '${LogNames.tempo}.state';
  static const config = '${LogNames.tempo}.config';

  // Vox, Flux, Codex subsystems
  static const vox = '${LogNames.tempo}.vox';
  static const flux = '${LogNames.tempo}.flux';
  static const codex = '${LogNames.tempo}.codex';
}

final Map<String, int> _loggerAccessCounter = {};
Logger _trackLoggerAccess(String name, Logger logger) {
  _loggerAccessCounter[name] = (_loggerAccessCounter[name] ?? 0) + 1;
  if (kDebugMode && _loggerAccessCounter[name]! > 1) {
    // ignore: avoid_print
    print(
      '  WARNING: Logger ${name} access ${_loggerAccessCounter[name]} times!',
    );
  }
  return logger;
}

final tempoLog = _trackLoggerAccess(LogNames.tempo, Logger(LogNames.tempo));
final tempoEditorLog = Logger(LogNames.editor);
final tempoTaskEditorLog = Logger(LogNames.taskEditor);
final tempoProjectEditorLog = Logger(LogNames.projectEditor);
final tempoMotionLog = Logger(LogNames.motionPlugin);
final tempoScaffoldingLog = Logger(LogNames.scaffoldingPlugin);
final tempoLayerLog = Logger(LogNames.editorLayer);
final tempoBlockHighlighterLog = Logger(LogNames.blockHighlighterLayer);
final tempoTapDelegateLog = Logger(LogNames.editorTapDelegate);
final tempoThingTapDelegateLog = Logger(LogNames.thingTapDelegate);
final tempoStyleLog = Logger(LogNames.style);
final tempoUILog = Logger(LogNames.ui);
final tempoPanelLog = _trackLoggerAccess(
  LogNames.uiPanel,
  Logger(LogNames.uiPanel),
);
final tempoRouteLog = Logger(LogNames.route);
final tempoTaskRouteLog = Logger(LogNames.taskRoute);
final tempoThingRouteLog = _trackLoggerAccess(
  LogNames.thingRoute,
  Logger(LogNames.thingRoute),
);
final tempoDBLog = _trackLoggerAccess(LogNames.db, Logger(LogNames.db));
final tempoDBSeedLog = Logger(LogNames.dbSeed);
final tempoDBQueryLog = Logger(LogNames.dbQuery);
final tempoDBSchemaLog = Logger(LogNames.dbSchema);
final tempoDevtoolsLog = Logger(LogNames.devtools);
final tempoStateLog = Logger(LogNames.state);
final tempoConfigLog = Logger(LogNames.config);
final tempoVoxLog = Logger(LogNames.vox);
final tempoFluxLog = Logger(LogNames.flux);
final tempoCodexLog = Logger(LogNames.codex);

// Re-export SuperEditor and FTL loggers for convenience
// SuperEditor
final superEditorLog = SE.editorLog;
final superEditorEditsLog = SE.editorEditsLog;
final superEditorPoliciesLog = SE.editorPoliciesLog;
final superEditorScrollingLog = SE.editorScrollingLog;
final superEditorGesturesLog = SE.editorGesturesLog;
final superEditorKeyLog = SE.editorKeyLog;
final superEditorIMELog = SE.editorImeLog;
final superEditorIMEConnectionLog = SE.editorImeConnectionLog;
final superEditorIMEDeltasLog = SE.editorImeDeltasLog;
final superEditoriOSFloatingCursorLog = SE.editorIosFloatingCursorLog;
final superEditorLayoutLog = SE.editorLayoutLog;
final superEditorStyleLog = SE.editorStyleLog;
final superEditorDocLog = SE.editorDocLog;
final superEditorOpsLog = SE.editorOpsLog;
final superEditorTokensLog = SE.editorTokensLog;
final superEditorTagsLog = SE.editorTagsLog;
final superEditorStableTagsLog = SE.editorStableTagsLog;
final superEditorPatternTagsLog = SE.editorPatternTagsLog;
final superEditorActionTagsLog = SE.editorActionTagsLog;
final superEditorSpellingAndGrammarLog = SE.editorSpellingAndGrammarLog;

// Super Reader
final superReaderLog = SE.readerLog;
final superReaderScrollingLog = SE.readerScrollingLog;
final superReaderGesturesLog = SE.readerGesturesLog;
final superReaderKeyLog = SE.readerKeyLog;
final superReaderImeLog = SE.readerImeLog;
final superReaderLayoutLog = SE.readerLayoutLog;
final superReaderStyleLog = SE.readerStyleLog;
final superReaderDocLog = SE.readerDocLog;
final superReaderOpsLog = SE.readerOpsLog;

// Text fields
final superTextFieldLog = SE.textFieldLog;
final superTextScrollingTextFieldLog = SE.scrollingTextFieldLog;
final superTextIMETextFieldLog = SE.imeTextFieldLog;
final superTextAndroidTextFieldLog = SE.androidTextFieldLog;
final superTextiOSTextFieldLog = SE.iosTextFieldLog;

// Infrastructure
final superDocGesturesLog = SE.docGesturesLog;
final superInfrastructureLog = SE.infrastructureLog;
final superKeyboardPanelLog = SE.keyboardPanelLog;
final superLongPressSelectionLog = SE.longPressSelectionLog;
final superSchedulerLog = SE.schedulerLog;
final superContentLayersLog = SE.contentLayersLog;
final superAttributionsLog = SE.attributionsLog;

// Message/Chat
final superChatPageElementLog = SE.messagePageElementLog;
final superChatPageLayoutLog = SE.messagePageLayoutLog;
final superChatPagePaintLog = SE.messagePagePaintLog;
final superChatEditorHeightLog = SE.messageEditorHeightLog;

// FTL
final FTLLeaderLog = FtlLogs.leader;
final FTLFollowerLog = FtlLogs.follower;
final FTLLinkLog = FtlLogs.link;
final FTLBoundaryLog = FtlLogs.boundary;
final FTLWidgetBoundaryLog = FtlLogs.widgetBoundary;

void _showLogMessage(LogRecord record) {
  final formatted = _formatLogRecord(record);

  if (kDebugMode) {
    print(formatted);
  }

  // Also use dart:developer log for IDE integration
  dev.log(
    formatted,
    name: record.loggerName,
    level: record.level.value,
    time: record.time,
  );
}

class LoggerConfig {
  static final Set<String> _configuredLoggers = {};
  static final Set<String> _listenersAttached = {};
  static bool _isInitialized = false;
  static Level _globalLevel = Level.FINEST;

  /// Configure a logger with level and listener (only adds listener once)
  static void _configureLogger(
    Logger logger,
    Level level, {
    bool attach = false,
  }) {
    // Skip if already configured
    if (_configuredLoggers.contains(logger.fullName)) {
      return;
    }
    _configuredLoggers.add(logger.fullName);
    if (kDebugMode) {
      print('DEBUG:  Initializing logger: ${logger.fullName}');
    }

    // Apply the level (respecting global minimum)
    final logLevel = _globalLevel > level ? _globalLevel : level;
    logger.level = logLevel;

    if (attach) {
      logger.onRecord.listen(_showLogMessage);
    }
  }

  static void initialize([Level? level]) {
    if (kDebugMode) {
      print('Configuring loggers...');
    }
    if (_isInitialized) return;
    _isInitialized = true;
    _globalLevel = level ?? Level.FINEST;

    // Enable hierarchical logging
    hierarchicalLoggingEnabled = true;
    Logger.root.level = Level.ALL;

    // Configure Tempo loggers
    _configureLogger(tempoLog, Level.FINEST, attach: true);
    _configureLogger(tempoEditorLog, Level.FINEST);
    _configureLogger(tempoTaskEditorLog, Level.FINER);
    _configureLogger(tempoProjectEditorLog, Level.FINEST);

    // SuperEditor plugins
    _configureLogger(tempoMotionLog, Level.INFO);
    _configureLogger(tempoScaffoldingLog, Level.INFO);

    // Custom SuperEditor layers
    _configureLogger(tempoLayerLog, Level.FINEST);
    _configureLogger(tempoBlockHighlighterLog, Level.FINEST);
    _configureLogger(tempoTapDelegateLog, Level.FINEST);
    _configureLogger(tempoThingTapDelegateLog, Level.FINER);

    // Styling
    _configureLogger(tempoStyleLog, Level.FINER);

    // Paneling
    _configureLogger(tempoUILog, Level.FINER);
    _configureLogger(tempoPanelLog, Level.FINER);

    // Routing
    _configureLogger(tempoRouteLog, Level.FINEST);
    _configureLogger(tempoTaskRouteLog, Level.FINER);
    _configureLogger(tempoThingRouteLog, Level.FINER);

    // Database
    _configureLogger(tempoDBLog, Level.INFO);
    _configureLogger(tempoDBSeedLog, Level.WARNING);
    _configureLogger(tempoDBQueryLog, Level.INFO);
    _configureLogger(tempoDBSchemaLog, Level.INFO);

    // Development tools
    _configureLogger(tempoDevtoolsLog, Level.INFO);
    _configureLogger(tempoStateLog, Level.FINEST);
    _configureLogger(tempoConfigLog, Level.FINEST);

    // Subsystems
    _configureLogger(tempoVoxLog, Level.INFO);
    _configureLogger(tempoFluxLog, Level.FINEST);
    _configureLogger(tempoCodexLog, Level.INFO);

    // Configure third-party loggers (use existing instances from packages)
    // SuperEditor
    _configureLogger(superEditorLog, Level.WARNING, attach: true);
    _configureLogger(superEditorEditsLog, Level.WARNING);
    _configureLogger(superEditorPoliciesLog, Level.WARNING);
    _configureLogger(superEditorScrollingLog, Level.WARNING);
    _configureLogger(superEditorGesturesLog, Level.WARNING);
    _configureLogger(superEditorKeyLog, Level.WARNING);
    _configureLogger(superEditorIMELog, Level.WARNING);
    _configureLogger(superEditorIMEConnectionLog, Level.WARNING);
    _configureLogger(superEditorIMEDeltasLog, Level.WARNING);
    _configureLogger(superEditoriOSFloatingCursorLog, Level.WARNING);
    _configureLogger(superEditorLayoutLog, Level.WARNING);
    _configureLogger(superEditorStyleLog, Level.WARNING);
    _configureLogger(superEditorDocLog, Level.WARNING);
    _configureLogger(superEditorOpsLog, Level.WARNING);
    _configureLogger(superEditorTokensLog, Level.WARNING);
    _configureLogger(superEditorTagsLog, Level.WARNING);
    _configureLogger(superEditorStableTagsLog, Level.WARNING);
    _configureLogger(superEditorPatternTagsLog, Level.WARNING);
    _configureLogger(superEditorActionTagsLog, Level.WARNING);
    _configureLogger(superEditorSpellingAndGrammarLog, Level.WARNING);

    // Super Reader
    _configureLogger(superReaderLog, Level.WARNING, attach: true);
    _configureLogger(superReaderScrollingLog, Level.WARNING);
    _configureLogger(superReaderGesturesLog, Level.WARNING);
    _configureLogger(superReaderKeyLog, Level.WARNING);
    _configureLogger(superReaderImeLog, Level.WARNING);
    _configureLogger(superReaderLayoutLog, Level.WARNING);
    _configureLogger(superReaderStyleLog, Level.WARNING);
    _configureLogger(superReaderDocLog, Level.WARNING);
    _configureLogger(superReaderOpsLog, Level.WARNING);

    // Text fields
    _configureLogger(superTextFieldLog, Level.WARNING, attach: true);
    _configureLogger(superTextScrollingTextFieldLog, Level.WARNING);
    _configureLogger(superTextIMETextFieldLog, Level.WARNING);
    _configureLogger(superTextAndroidTextFieldLog, Level.WARNING);
    _configureLogger(superTextiOSTextFieldLog, Level.WARNING);

    // Infrastructure
    _configureLogger(superDocGesturesLog, Level.WARNING, attach: true);
    _configureLogger(superInfrastructureLog, Level.WARNING, attach: true);
    _configureLogger(superKeyboardPanelLog, Level.WARNING);
    _configureLogger(superLongPressSelectionLog, Level.WARNING);
    _configureLogger(superSchedulerLog, Level.WARNING);
    _configureLogger(superContentLayersLog, Level.WARNING);
    _configureLogger(superAttributionsLog, Level.WARNING);

    // Message/Chat
    _configureLogger(Logger('chat.message'), Level.WARNING, attach: true);
    _configureLogger(superChatPageElementLog, Level.WARNING);
    _configureLogger(superChatPageLayoutLog, Level.WARNING);
    _configureLogger(superChatPagePaintLog, Level.WARNING);
    _configureLogger(superChatEditorHeightLog, Level.WARNING);

    // FTL
    _configureLogger(FTLLeaderLog, Level.SEVERE, attach: true);
    _configureLogger(FTLFollowerLog, Level.SEVERE, attach: true);
    _configureLogger(FTLLinkLog, Level.SEVERE, attach: true);
    _configureLogger(FTLBoundaryLog, Level.SEVERE, attach: true);
    _configureLogger(FTLWidgetBoundaryLog, Level.SEVERE, attach: true);
  }

  /// Update log level for a specific logger at runtime
  static void setLogLevel(Logger logger, Level level) {
    logger.level = level;
  }

  /// Update multiple log levels at once
  static void setLogLevels(Map<Logger, Level> levels) {
    levels.forEach((logger, level) {
      logger.level = level;
    });
  }
}

/// ANSI color codes for terminal output
class LogColors {
  static const reset = '\x1B[0m';
  static const bold = '\x1B[1m';
  static const dim = '\x1B[2m';

  // Level colors
  static const finest = '\x1B[32m'; // Green
  static const finer = '\x1B[36m'; // Cyan
  static const fine = '\x1B[34m'; // Blue
  static const info = '\x1B[37m'; // White
  static const warning = '\x1B[33m'; // Yellow
  static const severe = '\x1B[31m'; // Red
  static const shout = '\x1B[35m'; // Magenta

  // Component colors
  static const logger = '\x1B[90m'; // Gray
}

String _colorizeLevel(Level level) {
  final color = switch (level) {
    Level.FINEST => LogColors.finest,
    Level.FINER => LogColors.finer,
    Level.FINE => LogColors.fine,
    Level.INFO => LogColors.info,
    Level.WARNING => LogColors.warning,
    Level.SEVERE => LogColors.severe,
    Level.SHOUT => LogColors.shout,
    _ => LogColors.info,
  };
  return '$color${level.name.padLeft(7)}${LogColors.reset}';
}

String _formatLogRecord(LogRecord record) {
  final timestamp = record.time.toIso8601String().substring(
    11,
    23,
  ); // HH:mm:ss.SSS
  final level = _colorizeLevel(record.level);
  final logger = '${LogColors.logger}${record.loggerName}${LogColors.reset}';
  final message = record.message;

  return '$level $logger: $message';
}
