import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../objectbox/services/thing_service.dart';
import '../../things/project/project_editor.dart';
import '../../things/task/task_editor.dart';
import '../../things/thing/interface.dart';
import '../../things/thing/thing_entity.dart';
import '../../things/thing_factories.dart';
import '../panel_content_registry.dart';
import '../panel_request.dart';
import '../panel_manager.dart';
import '../../editor/editor.dart';
import '../../infrastructure/logging.dart';
import '../widgets/panel/panel_appbar.dart';
import '../../things/task/task_repository.dart';
import '../../things/project/project_repository.dart';
import 'package:objectbox/objectbox.dart';

/// Universal detail editor that works for any Thing entity
class DetailEditorBuilder extends PanelContentBuilder {
  @override
  bool canBuild(String entityType) {
    // Can build detail view for any entity type
    return true;
  }

  @override
  Widget build(BuildContext context, String uuid, WidgetRef ref) {
    return PanelWrappedEditor(uuid: uuid);
  }
}

/// Panel-wrapped editor that uses the same content as mobile pages
class PanelWrappedEditor extends ConsumerWidget {
  const PanelWrappedEditor({super.key, required this.uuid});

  final String uuid;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Look up the entity from the thing service
    final thingService = ref.read(thingServiceProvider);
    final thingEntry = thingService.lookup(uuid);

    if (thingEntry == null) {
      // UUID not found in registry
      return _buildEntityNotFoundScreen(context, uuid);
    }

    // Return appropriate editor based on entity type
    // This mirrors the logic from EntityPage but wrapped for panel use
    switch (thingEntry.type) {
      case 'task':
        try {
          final task = ref.read(taskRepositoryProvider).getByUUID(uuid);
          if (task != null) {
            // Use the same TaskEditor as mobile, but without AppBar
            return TaskEditor.forPanel(
              task: task,
              projectID: task.project.targetId,
            );
          }
        } catch (e) {
          // Task not found - possibly deleted but registry not updated
        }
        break;

      case 'project':
        try {
          final project = ref.read(projectRepositoryProvider).getByUUID(uuid);
          if (project != null) {
            // Use the same ProjectEditor as mobile, but without AppBar
            return ProjectEditor.forPanel(project: project);
          }
        } catch (e) {
          // Project not found - possibly deleted but registry not updated
        }
        break;

      case 'activity':
        // TODO: Implement ActivityEditor when needed
        return _buildUnsupportedTypeScreen(context, 'activity');

      default:
        // Unknown entity type
        return _buildUnsupportedTypeScreen(context, thingEntry.type);
    }

    // If we get here, the entity was found in registry but not in the actual database
    return _buildEntityNotFoundScreen(context, uuid, thingEntry.type);
  }

  Widget _buildEntityNotFoundScreen(
    BuildContext context,
    String uuid, [
    String? entityType,
  ]) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          const Icon(Icons.search_off, size: 64, color: Colors.grey),
          const SizedBox(height: 16),
          const Text(
            'Entity not found',
            style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 8),
          Text('UUID: $uuid', style: const TextStyle(color: Colors.grey)),
          if (entityType != null) ...[
            const SizedBox(height: 4),
            Text(
              'Type: $entityType',
              style: const TextStyle(color: Colors.grey),
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildUnsupportedTypeScreen(BuildContext context, String entityType) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          const Icon(Icons.extension_off, size: 64, color: Colors.grey),
          const SizedBox(height: 16),
          const Text(
            'Entity type not supported',
            style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 8),
          Text('Type: $entityType', style: const TextStyle(color: Colors.grey)),
        ],
      ),
    );
  }
}

/// Legacy detail editor widget (keeping for compatibility)
class DetailEditor extends ConsumerStatefulWidget {
  const DetailEditor({super.key, required this.uuid});

  final String uuid;

  @override
  ConsumerState<DetailEditor> createState() => _DetailEditorState();
}

class _DetailEditorState extends ConsumerState<DetailEditor> {
  late MutableDocument _document;
  late MutableDocumentComposer _composer;
  late HoverManager _hoverManager;
  late LeaderLinkManager _linkManager;
  final ScrollController _scrollController = ScrollController();

  ThingEntity? _entity;
  bool _isLoading = true;
  String? _error;

  // Cache to prevent rebuilds during resize operations
  Widget? _cachedSuperEditor;
  bool _lastResizeState = false;

  @override
  void initState() {
    super.initState();
    _hoverManager = HoverManager();
    _linkManager = LeaderLinkManager();
    _initializeEditor();
  }

  Future<void> _initializeEditor() async {
    try {
      // Load the entity
      final thingService = ref.read(thingServiceProvider);
      _entity = thingService.lookup(widget.uuid);

      if (_entity == null) {
        setState(() {
          _error = 'Entity not found: ${widget.uuid}';
          _isLoading = false;
        });
        return;
      }

      // Initialize document
      _document = MutableDocument();
      _composer = MutableDocumentComposer();

      // Create header node based on entity type
      final headerNode = _createHeaderNode(_entity!);
      _document.add(headerNode);

      // Add entity content
      await _loadEntityContent(_entity!);

      // Editor will be created by createSuperEditor

      setState(() {
        _isLoading = false;
      });
    } catch (e, stack) {
      tempoLog.severe('Error initializing detail editor', e, stack);
      setState(() {
        _error = 'Error loading entity: $e';
        _isLoading = false;
      });
    }
  }

  DocumentNode _createHeaderNode(ThingEntity entity) {
    // Create appropriate header based on entity type
    String icon;
    AttributedText title;

    switch (entity.type) {
      case 'Task':
        icon = 'task';
        title = AttributedText('Task Details');
        break;
      case 'Project':
        icon = 'folder';
        title = AttributedText('Project Details');
        break;
      default:
        icon = 'description';
        title = AttributedText(entity.type);
    }

    return EditableTextNodeFactory.documentHeader(
      id: Editor.createNodeId(),
      title: title,
      icon: icon,
      isEditable: false,
    );
  }

  Future<void> _loadEntityContent(ThingEntity entity) async {
    // Get the appropriate repository for this entity type
    final thingRepo = _getRepositoryForType(entity.type);
    if (thingRepo == null) {
      // Generic entity content for unsupported types
      _document.add(
        ParagraphNode(
          id: Editor.createNodeId(),
          text: AttributedText(
            'Entity type: ${entity.type}\nUUID: ${entity.uuid}',
          ),
        ),
      );
      return;
    }

    // Get the entity and load its children
    final actualEntity = thingRepo.byUUID(entity.uuid);
    if (actualEntity == null) {
      tempoLog.warning('Entity not found: ${entity.uuid}');
      return;
    }

    // Load children content using the generic approach
    await _loadChildrenContent(actualEntity.children);
  }

  /// Get the appropriate repository for an entity type
  ThingRepository? _getRepositoryForType(String type) {
    switch (type) {
      case 'Task':
        return ref.read(taskRepositoryProvider);
      case 'Project':
        return ref.read(projectRepositoryProvider);
      // Add more types as needed
      default:
        return null;
    }
  }

  /// Load children content into the document
  Future<void> _loadChildrenContent(ToMany<ThingEntity> children) async {
    if (children.isEmpty) {
      // If no children, add a placeholder paragraph for editing
      _document.add(
        ParagraphNode(
          id: Editor.createNodeId(),
          text: AttributedText('Start writing...'),
        ),
      );
      return;
    }

    // Load each child as a document node using the factory registry
    for (final child in children) {
      try {
        final node = await _createNodeFromChild(child);
        if (node != null) {
          _document.add(node);
        }
      } catch (e, stack) {
        tempoLog.warning('Failed to load child node: ${child.uuid}', e, stack);
        // Add error placeholder
        _document.add(
          ParagraphNode(
            id: Editor.createNodeId(),
            text: AttributedText('[Error loading content: ${child.type}]'),
          ),
        );
      }
    }
  }

  /// Create a document node from a child entity using the node factory registry
  Future<DocumentNode?> _createNodeFromChild(ThingEntity child) async {
    final registry = ref.read(nodeFactoryRegistryProvider);

    try {
      final node = await registry.toNode(child);
      if (node != null) {
        return node;
      }

      // Fallback to generic paragraph for unsupported types
      return ParagraphNode(
        id: Editor.createNodeId(),
        text: AttributedText('${child.type}: ${child.uuid}'),
      );
    } catch (e, stack) {
      tempoLog.warning(
        'Failed to create node from child entity: ${child.uuid}',
        e,
        stack,
      );
      return ParagraphNode(
        id: Editor.createNodeId(),
        text: AttributedText('[Error: ${child.type}]'),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final screenWidth = MediaQuery.of(context).size.width;
    final isMobile = screenWidth <= 768;
    final panelState = ref.watch(panelManagerProvider);
    final isResizing = panelState.isResizing;

    if (_isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_error != null) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(Icons.error_outline, size: 48, color: Colors.red),
            const SizedBox(height: 16),
            Text(_error!),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: () => _initializeEditor(),
              child: const Text('Retry'),
            ),
          ],
        ),
      );
    }

    // Cache SuperEditor to avoid rebuilds during resize
    if (!isResizing &&
        (_cachedSuperEditor == null || _lastResizeState != isResizing)) {
      try {
        _cachedSuperEditor = RepaintBoundary(
          child: createSuperEditor(
            context,
            _document,
            _composer,
            hoverManager: _hoverManager,
            linkManager: _linkManager,
            taskRepository: ref.read(taskRepositoryProvider),
            ref: ref,
            scrollController: _scrollController,
          ),
        );
        _lastResizeState = isResizing;
      } catch (e) {
        tempoLog.severe('Error creating SuperEditor: $e');
        _cachedSuperEditor = Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Icon(Icons.error, size: 48, color: Colors.red),
              const SizedBox(height: 16),
              Text('Error loading content: $e'),
              const SizedBox(height: 16),
              ElevatedButton(
                onPressed: _initializeEditor,
                child: const Text('Retry'),
              ),
            ],
          ),
        );
      }
    }

    final superEditor =
        _cachedSuperEditor ?? const Center(child: CircularProgressIndicator());

    final content = Container(
      color: Theme.of(context).colorScheme.surfaceContainer,
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 800),
          child: Column(children: [Expanded(child: superEditor)]),
        ),
      ),
    );

    final headerSize = PanelAppBar.computePreferredSize(context);
    final header = PanelAppBar(
      title: _getEntityTitle(),
      size: headerSize,
      scrollController: _scrollController,
      actions: [
        IconButton(
          icon: const Icon(Icons.close, size: 16),
          onPressed:
              () => ref
                  .read(panelManagerProvider.notifier)
                  .emit(ClosePanelRequest()),
          tooltip: 'Close panel',
          iconSize: 16,
          padding: const EdgeInsets.all(4),
          constraints: const BoxConstraints(minWidth: 24, minHeight: 24),
        ),
      ],
    );

    // Return layout based on mobile/desktop
    if (isMobile) {
      return Scaffold(appBar: header, body: content);
    }

    // Desktop layout with fixed app bar
    return Scaffold(
      appBar: header,
      body: Column(children: [Expanded(child: content)]),
    );
  }

  String _getEntityTitle() {
    if (_entity == null) return 'Detail';

    switch (_entity!.type) {
      case 'Task':
        return 'Task Details';
      case 'Project':
        return 'Project Details';
      default:
        return '${_entity!.type} Details';
    }
  }
}
