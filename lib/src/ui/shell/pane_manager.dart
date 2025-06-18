import 'package:flutter/material.dart';

import '../../infrastructure/logging.dart';

/// Global pane manager instance
final paneManager = PaneManager();

/// Manages the visibility and state of different panes in the shell
class PaneManager extends ChangeNotifier {
  bool _showSidebar = true;
  bool _showDetailPane = false;
  String? _detailPaneRoute;
  Map<String, dynamic>? _detailPaneArgs;

  Map<String, dynamic>? get detailPaneArgs => _detailPaneArgs;
  String? get detailPaneRoute => _detailPaneRoute;
  bool get showDetailPane => _showDetailPane;
  bool get showSidebar => _showSidebar;

  void closeDetailPane() {
    _detailPaneRoute = null;
    _detailPaneArgs = null;
    _showDetailPane = false;
    tempoLog.fine('Detail pane closed');
    notifyListeners();
  }

  void openDetailPane(String route, {Map<String, dynamic>? args}) {
    _detailPaneRoute = route;
    _detailPaneArgs = args;
    _showDetailPane = true;
    tempoLog.fine('Detail pane opened: $route');
    notifyListeners();
  }

  void setSidebarVisible(bool visible) {
    if (_showSidebar != visible) {
      _showSidebar = visible;
      notifyListeners();
    }
  }

  void toggleSidebar() {
    _showSidebar = !_showSidebar;
    tempoLog.fine('Sidebar toggled: $_showSidebar');
    notifyListeners();
  }
}
