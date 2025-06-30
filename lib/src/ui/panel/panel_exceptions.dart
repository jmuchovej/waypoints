/// Custom exceptions for panel system
class PanelSystemException implements Exception {
  const PanelSystemException(this.message, [this.context]);
  
  final String message;
  final String? context;
  
  @override
  String toString() => context != null ? '$message: $context' : message;
}

/// Exception thrown when panel depth exceeds reasonable limits
class PanelDepthExceededException extends PanelSystemException {
  const PanelDepthExceededException(this.currentDepth, [this.maxRecommended = 20])
      : super('Panel depth limit exceeded');
  
  final int currentDepth;
  final int maxRecommended;
  
  @override
  String toString() => 
      'Panel depth of $currentDepth exceeds recommended limit of $maxRecommended. '
      'Consider reducing the number of nested panels for better performance.';
}

/// Exception thrown when navigation state becomes corrupted
class PanelNavigationException extends PanelSystemException {
  const PanelNavigationException(String message, this.currentPath)
      : super(message);
  
  final String currentPath;
  
  @override
  String toString() => '$message (current path: $currentPath)';
}