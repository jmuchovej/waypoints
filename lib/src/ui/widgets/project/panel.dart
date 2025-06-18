import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:auto_route/auto_route.dart';

import '../../../editor/editor.dart';
import '../../../infrastructure/logging.dart';
import '../../../router.dart';
import '../../../things/project/editor/project_node.dart';
import '../../shell/pane_manager.dart';
import '../../../things/project/project_repository.dart';

/// Panel for displaying projects list using SuperEditor
/// This is specifically designed for the /routes/project/index page
class ProjectPanel extends ConsumerStatefulWidget {
  const ProjectPanel({super.key});

  @override
  ConsumerState<ProjectPanel> createState() => _ProjectPanelState();
}

class _ProjectPanelState extends ConsumerState<ProjectPanel> {
  late MutableDocument _document;
  late MutableDocumentComposer _composer;
  late HoverManager _hoverManager;
  late LeaderLinkManager _linkManager;
  final ScrollController _scrollController = ScrollController();
  bool _isLoading = true;
  bool _showAppBarTitle = false;
  String? _documentHeaderNodeId;

  @override
  void initState() {
    super.initState();

    // Listen to pane manager changes
    paneManager.addListener(_onPaneStateChanged);

    // Listen to scroll changes
    _scrollController.addListener(_onScroll);

    _initializePanel();
  }

  @override
  void dispose() {
    paneManager.removeListener(_onPaneStateChanged);
    _scrollController.removeListener(_onScroll);
    _scrollController.dispose();
    super.dispose();
  }

  void _initializePanel() async {
    try {
      // Initialize managers
      _hoverManager = HoverManager();
      _linkManager = LeaderLinkManager();

      // Initialize document
      await _initializeDocument();

      setState(() {
        _isLoading = false;
      });
    } catch (e) {
      fluxLog.severe('Error initializing projects panel: $e');
      setState(() {
        _isLoading = false;
      });
    }
  }

  Future<void> _initializeDocument() async {
    // Create empty document
    final nodeId = Editor.createNodeId();
    _document = MutableDocument.empty(nodeId);
    _document.deleteNode(nodeId);

    // Load projects and add to document
    await _loadProjects();

    // Initialize composer
    _composer = MutableDocumentComposer();
  }

  Future<void> _loadProjects() async {
    try {
      final projectRepo = ref.read(projectRepositoryProvider);
      final projects = projectRepo.active(); // Only show active projects

      fluxLog.info('Loaded ${projects.length} active projects');

      // Sort projects by modification date (newest first)
      projects.sort((a, b) => b.modifyDate.compareTo(a.modifyDate));

      // Add projects to document as ProjectNodes
      for (final project in projects) {
        final projectNode = ProjectNode.fromProject(project);
        _document.add(projectNode);
      }

      fluxLog.info('Added ${projects.length} project nodes to document');
    } catch (e) {
      fluxLog.severe('Error loading projects: $e');
    }
  }

  Future<void> _refreshProjects() async {
    setState(() {
      _isLoading = true;
    });

    // Clear existing document (except header)
    final headerNode = _document.firstOrNull;
    _document.deleteAllButHeader();
    if (headerNode is DocumentHeaderNode) {
      _document.insertNodeAt(0, headerNode);
    }

    // Reload projects
    await _loadProjects();

    setState(() {
      _isLoading = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final screenWidth = MediaQuery.of(context).size.width;
    final isMobile = screenWidth <= 768;

    // Add document header for desktop if not already present
    final shouldAddHeader =
        !isMobile &&
        (_document.isEmpty ||
            (_document.firstOrNull != null &&
                _document.firstOrNull is! DocumentHeaderNode));

    if (shouldAddHeader) {
      final nodeId = Editor.createNodeId();
      final headerNode = DocumentHeaderNode(id: nodeId, title: 'Projects');

      _document.insertNodeAt(0, headerNode);
      _documentHeaderNodeId = nodeId;
    }

    // Create SuperEditor
    Widget superEditor;
    if (_isLoading) {
      superEditor = const Center(child: CircularProgressIndicator());
    } else if (_document.isEmpty ||
        (_document.length == 1 &&
            _document.firstOrNull is DocumentHeaderNode)) {
      superEditor = _buildEmptyState();
    } else {
      try {
        superEditor = createSuperEditor(
          context,
          _document,
          _composer,
          hoverManager: _hoverManager,
          linkManager: _linkManager,
          scrollController: _scrollController,
        );
      } catch (e) {
        fluxLog.severe('Error creating SuperEditor: $e');
        superEditor = Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Icon(Icons.error, size: 48, color: Colors.red),
              const SizedBox(height: 16),
              Text('Error loading projects: $e'),
              const SizedBox(height: 16),
              ElevatedButton(
                onPressed: _refreshProjects,
                child: const Text('Retry'),
              ),
            ],
          ),
        );
      }
    }

    final content = Container(
      color: Theme.of(context).colorScheme.surfaceContainer,
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 800),
          child: Column(children: [Expanded(child: superEditor)]),
        ),
      ),
    );

    // Build app bar
    final appBar = AppBar(
      backgroundColor: Theme.of(context).colorScheme.surfaceContainer,
      elevation: 0,
      scrolledUnderElevation: 0,
      title: AnimatedSlide(
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeInOut,
        offset: Offset(0, _showAppBarTitle ? 0 : 1),
        child: AnimatedOpacity(
          duration: const Duration(milliseconds: 200),
          opacity: _showAppBarTitle ? 1.0 : 0.0,
          child: const Text('Projects'),
        ),
      ),
      actions: [
        IconButton(
          icon: const Icon(Icons.refresh),
          onPressed: _refreshProjects,
          tooltip: 'Refresh',
        ),
        IconButton(
          icon: const Icon(Icons.add),
          onPressed: () => _createNewProject(),
          tooltip: 'New Project',
        ),
      ],
    );

    // Return layout based on mobile/desktop
    if (isMobile) {
      return Scaffold(appBar: appBar, body: content);
    }

    // Desktop layout with fixed app bar
    return Scaffold(
      body: Column(
        children: [
          appBar,
          AnimatedContainer(
            duration: const Duration(milliseconds: 200),
            height: 1,
            color:
                _showAppBarTitle
                    ? Theme.of(context).colorScheme.outlineVariant
                    : Theme.of(context).colorScheme.surfaceContainer,
          ),
          Expanded(child: content),
        ],
      ),
    );
  }

  Widget _buildEmptyState() {
    final theme = Theme.of(context);

    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.folder_open,
            size: 80,
            color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.6),
          ),
          const SizedBox(height: 16),
          Text(
            'No projects yet',
            style: theme.textTheme.headlineSmall?.copyWith(
              fontWeight: FontWeight.bold,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'Create your first project to organize your tasks',
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 24),
          ElevatedButton.icon(
            icon: const Icon(Icons.add),
            label: const Text('Create Project'),
            onPressed: _createNewProject,
          ),
        ],
      ),
    );
  }

  void _createNewProject() async {
    await context.router.push(CreateProjectRoute());
    // Refresh projects after creation
    await _refreshProjects();
  }

  void _onPaneStateChanged() {
    setState(() {});
  }

  void _onScroll() {
    if (_documentHeaderNodeId == null) return;

    final shouldShowTitle = _scrollController.offset > 100;

    if (shouldShowTitle != _showAppBarTitle) {
      setState(() {
        _showAppBarTitle = shouldShowTitle;
      });
    }
  }
}
