import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../infrastructure/logging.dart';
import '../../things/task/task_repository.dart';
import '../../things/project/project_repository.dart';
import '../../objectbox/services/thing_service.dart';
import '../../things/task/task_editor.dart';
import '../../things/project/project_editor.dart';

/// Panel context information for editors
class PanelInfo {
  final int panelIndex; // 0 for base content, 1+ for actual panels
  final String? uuid; // UUID of the entity this panel is showing

  const PanelInfo({required this.panelIndex, this.uuid});

  // Convenience constructors
  const PanelInfo.baseContent({String? uuid}) : panelIndex = 0, uuid = uuid;
  const PanelInfo.panel(this.panelIndex, this.uuid);

  @override
  String toString() => 'PanelInfo(index: $panelIndex, uuid: $uuid)';
}

class EntityPage extends ConsumerWidget {
  const EntityPage({
    super.key,
    required this.uuid,
    this.panelInfo = const PanelInfo.baseContent(),
  });

  final String uuid;
  final PanelInfo panelInfo;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Look up the entity from the thing service
    final thingService = ref.read(thingServiceProvider);
    final thingEntry = thingService.lookup(uuid);
    tempoLog.info('EntityPage.build: uuid=$uuid, panelInfo=$panelInfo');

    if (thingEntry == null) {
      // UUID not found in registry
      return _buildEntityNotFoundScreen(context, uuid);
    }

    // Get the main content based on entity type
    Widget mainContent;
    String title = 'Unknown';

    // Return appropriate editor based on entity type
    switch (thingEntry.type) {
      case 'task':
        try {
          final task = ref.read(taskRepositoryProvider).getByUUID(uuid);
          if (task != null) {
            title = task.title.toPlainText() ?? 'Untitled Task';
            mainContent = TaskEditor(
              task: task,
              projectID: task.project.targetId,
              panelInfo: panelInfo,
            );
          } else {
            return _buildEntityNotFoundScreen(context, uuid, thingEntry.type);
          }
        } catch (e) {
          // Task not found - possibly deleted but registry not updated
          return _buildEntityNotFoundScreen(context, uuid, thingEntry.type);
        }
        break;

      case 'project':
        try {
          final project = ref.read(projectRepositoryProvider).getByUUID(uuid);
          if (project != null) {
            title = project.title.toPlainText() ?? 'Untitled Project';
            mainContent = ProjectEditor(
              project: project,
              panelInfo: panelInfo,
            );
          } else {
            return _buildEntityNotFoundScreen(context, uuid, thingEntry.type);
          }
        } catch (e) {
          // Project not found - possibly deleted but registry not updated
          return _buildEntityNotFoundScreen(context, uuid, thingEntry.type);
        }
        break;

      case 'activity':
        // TODO: Implement ActivityEditor when needed
        return _buildUnsupportedTypeScreen(context, 'activity');

      default:
        // Unknown entity type
        return _buildUnsupportedTypeScreen(context, thingEntry.type);
    }

    // Check if we can go back (has navigation stack)
    final canPop = Navigator.of(context).canPop();

    // Wrap with Scaffold and custom app bar for traditional navigation
    return Scaffold(
      appBar: AppBar(
        title: Text(title),
        backgroundColor: Colors.transparent,
        elevation: 0,
        leading: canPop
          ? IconButton(
              icon: const Icon(Icons.arrow_back),
              onPressed: () => Navigator.of(context).pop(),
            )
          : null,
        automaticallyImplyLeading: canPop,
      ),
      body: mainContent,
    );
  }

  Widget _buildEntityNotFoundScreen(
    BuildContext context,
    String uuid, [
    String? entityType,
  ]) {
    return Scaffold(
      appBar: AppBar(title: const Text('Entity Not Found')),
      body: Center(
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
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: () => context.go('/tasks'),
              child: const Text('Go to Tasks'),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildUnsupportedTypeScreen(BuildContext context, String entityType) {
    return Scaffold(
      appBar: AppBar(title: const Text('Unsupported Entity')),
      body: Center(
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
            Text(
              'Type: $entityType',
              style: const TextStyle(color: Colors.grey),
            ),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: () => context.go('/tasks'),
              child: const Text('Go to Tasks'),
            ),
          ],
        ),
      ),
    );
  }
}
