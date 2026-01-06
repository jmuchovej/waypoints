// ignore_for_file: non_constant_identifier_names

import 'dart:developer' as dev;

import 'package:flutter/foundation.dart';
import 'package:logging/logging.dart';

export 'package:logging/logging.dart'
    show Logger, Level, hierarchicalLoggingEnabled;

import 'package:follow_the_leader/follow_the_leader.dart' show FtlLogs;
import 'package:super_editor/super_editor.dart' as SE hide Logger;

class LogNames {
  static const waypoints = 'waypoints';

  static const editor = '${LogNames.waypoints}.editor';
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
  static const style = '${LogNames.waypoints}.style';

  // Paneling
  static const ui = '${LogNames.waypoints}.ui';
  static const uiPanel = '${LogNames.ui}.panel';

  // Routing
  static const route = '${LogNames.waypoints}.route';
  static const taskRoute = '${LogNames.route}.task';
  static const thingRoute = '${LogNames.route}.thing';

  // Database subsystem
  static const db = '${LogNames.waypoints}.db';
  static const dbSeed = '${LogNames.db}.seed';
  static const dbQuery = '${LogNames.db}.query';
  static const dbSchema = '${LogNames.db}.schema';

  // Development tools
  static const devtools = '${LogNames.waypoints}.devtools';

  static const state = '${LogNames.waypoints}.state';
  static const config = '${LogNames.waypoints}.config';

  // Vox, Flux, Codex subsystems
  static const vox = '${LogNames.waypoints}.vox';
  static const flux = '${LogNames.waypoints}.flux';
  static const codex = '${LogNames.waypoints}.codex';
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

final waypointsLog = _trackLoggerAccess(LogNames.waypoints, Logger(LogNames.waypoints));
final waypointsEditorLog = Logger(LogNames.editor);
final waypointsTaskEditorLog = Logger(LogNames.taskEditor);
final waypointsProjectEditorLog = Logger(LogNames.projectEditor);
final waypointsMotionLog = Logger(LogNames.motionPlugin);
final waypointsScaffoldingLog = Logger(LogNames.scaffoldingPlugin);
final waypointsLayerLog = Logger(LogNames.editorLayer);
final waypointsBlockHighlighterLog = Logger(LogNames.blockHighlighterLayer);
final waypointsTapDelegateLog = Logger(LogNames.editorTapDelegate);
final waypointsThingTapDelegateLog = Logger(LogNames.thingTapDelegate);
final waypointsStyleLog = Logger(LogNames.style);
final waypointsUILog = Logger(LogNames.ui);
final waypointsPanelLog = _trackLoggerAccess(
  LogNames.uiPanel,
  Logger(LogNames.uiPanel),
);
final waypointsRouteLog = Logger(LogNames.route);
final waypointsTaskRouteLog = Logger(LogNames.taskRoute);
final waypointsThingRouteLog = _trackLoggerAccess(
  LogNames.thingRoute,
  Logger(LogNames.thingRoute),
);
final waypointsDBLog = _trackLoggerAccess(LogNames.db, Logger(LogNames.db));
final waypointsDBSeedLog = Logger(LogNames.dbSeed);
final waypointsDBQueryLog = Logger(LogNames.dbQuery);
final waypointsDBSchemaLog = Logger(LogNames.dbSchema);
final waypointsDevtoolsLog = Logger(LogNames.devtools);
final waypointsStateLog = Logger(LogNames.state);
final waypointsConfigLog = Logger(LogNames.config);
final waypointsVoxLog = Logger(LogNames.vox);
final waypointsFluxLog = Logger(LogNames.flux);
final waypointsCodexLog = Logger(LogNames.codex);

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

    // Configure Waypoints loggers
    _configureLogger(waypointsLog, Level.FINEST, attach: true);
    _configureLogger(waypointsEditorLog, Level.FINEST);
    _configureLogger(waypointsTaskEditorLog, Level.FINER);
    _configureLogger(waypointsProjectEditorLog, Level.FINEST);

    // SuperEditor plugins
    _configureLogger(waypointsMotionLog, Level.INFO);
    _configureLogger(waypointsScaffoldingLog, Level.INFO);

    // Custom SuperEditor layers
    _configureLogger(waypointsLayerLog, Level.FINEST);
    _configureLogger(waypointsBlockHighlighterLog, Level.FINEST);
    _configureLogger(waypointsTapDelegateLog, Level.FINEST);
    _configureLogger(waypointsThingTapDelegateLog, Level.FINER);

    // Styling
    _configureLogger(waypointsStyleLog, Level.FINER);

    // Paneling
    _configureLogger(waypointsUILog, Level.FINER);
    _configureLogger(waypointsPanelLog, Level.FINER);

    // Routing
    _configureLogger(waypointsRouteLog, Level.FINEST);
    _configureLogger(waypointsTaskRouteLog, Level.FINER);
    _configureLogger(waypointsThingRouteLog, Level.FINER);

    // Database
    _configureLogger(waypointsDBLog, Level.FINEST);
    _configureLogger(waypointsDBSeedLog, Level.FINEST);
    _configureLogger(waypointsDBQueryLog, Level.INFO);
    _configureLogger(waypointsDBSchemaLog, Level.INFO);

    // Development tools
    _configureLogger(waypointsDevtoolsLog, Level.INFO);
    _configureLogger(waypointsStateLog, Level.FINEST);
    _configureLogger(waypointsConfigLog, Level.FINEST);

    // Subsystems
    _configureLogger(waypointsVoxLog, Level.INFO);
    _configureLogger(waypointsFluxLog, Level.FINEST);
    _configureLogger(waypointsCodexLog, Level.INFO);

    // Configure third-party loggers (use existing instances from packages)
    // SuperEditor
    _configureLogger(superEditorLog, Level.WARNING, attach: true);
    _configureLogger(superEditorEditsLog, Level.FINEST);
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
    _configureLogger(superEditorDocLog, Level.FINEST);
    _configureLogger(superEditorOpsLog, Level.FINEST);
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
