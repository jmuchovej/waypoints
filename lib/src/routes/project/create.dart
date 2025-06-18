import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:uuid/uuid.dart';

import '../../editor/editor.dart';
import '../../router.dart';
import '../../things/project/project_entity.dart';
import '../../things/project/project_repository.dart';

@RoutePage()
class CreateProjectScreen extends ConsumerStatefulWidget {
  const CreateProjectScreen({super.key});

  @override
  ConsumerState<CreateProjectScreen> createState() =>
      _CreateProjectScreenState();
}

class _CreateProjectScreenState extends ConsumerState<CreateProjectScreen> {
  late AttributedTextEditingController _titleController;
  late AttributedTextEditingController _descriptionController;
  late FocusNode _titleFocusNode;
  late FocusNode _descriptionFocusNode;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      backgroundColor: theme.colorScheme.surface,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        elevation: 0,
        leading: IconButton(
          icon: const Icon(Icons.close),
          onPressed: () {
            if (context.router.canPop()) {
              context.router.pop();
            } else {
              context.gotoProjects();
            }
          },
        ),
        actions: [
          TextButton.icon(
            icon: const Icon(Icons.save),
            label: const Text('Create'),
            onPressed: _createProject,
          ),
        ],
      ),
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 800),
          child: SingleChildScrollView(
            padding: const EdgeInsets.symmetric(
              horizontal: 24.0,
              vertical: 20.0,
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Title section
                Text(
                  'New Project',
                  style: theme.textTheme.displayMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 32),

                // Project title
                _buildTitleField(theme),
                const SizedBox(height: 24),

                // Project description
                _buildDescriptionField(theme),
              ],
            ),
          ),
        ),
      ),
    );
  }

  @override
  void dispose() {
    _titleController.dispose();
    _descriptionController.dispose();
    _titleFocusNode.dispose();
    _descriptionFocusNode.dispose();
    super.dispose();
  }

  @override
  void initState() {
    super.initState();
    _titleController = AttributedTextEditingController();
    _descriptionController = AttributedTextEditingController();
    _titleFocusNode = FocusNode(debugLabel: "project.create.title");
    _descriptionFocusNode = FocusNode(debugLabel: "project.create.description");
  }

  Widget _buildDescriptionField(ThemeData theme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Description (Optional)',
          style: theme.textTheme.titleMedium?.copyWith(
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: 8),
        Container(
          height: 200,
          decoration: BoxDecoration(
            border: Border.all(
              color: theme.colorScheme.outline.withValues(alpha: 0.2),
            ),
            borderRadius: BorderRadius.circular(8),
          ),
          child: Padding(
            padding: const EdgeInsets.all(12.0),
            child: TempoTextField(
              focusNode: _descriptionFocusNode,
              controller: _descriptionController,
              hintText: AttributedText('Describe your project...'),
              maxLines: null,
              textStyleBuilder: (attributions) {
                return TextStyle(
                  fontSize: 16,
                  color: theme.colorScheme.onSurface,
                );
              },
              hintStyleBuilder: (attributions) {
                return TextStyle(
                  fontSize: 16,
                  color: theme.colorScheme.onSurface.withValues(alpha: 0.4),
                );
              },
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildTitleField(ThemeData theme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Project Title',
          style: theme.textTheme.titleMedium?.copyWith(
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: 8),
        TempoTextField(
          focusNode: _titleFocusNode,
          controller: _titleController,
          hintText: AttributedText('Enter project title'),
          textStyleBuilder: (attributions) {
            return TextStyle(fontSize: 18, color: theme.colorScheme.onSurface);
          },
          hintStyleBuilder: (attributions) {
            return TextStyle(
              fontSize: 18,
              color: theme.colorScheme.onSurface.withValues(alpha: 0.4),
            );
          },
        ),
      ],
    );
  }

  void _createProject() {
    if (_titleController.text.toPlainText().trim().isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Project title cannot be empty')),
      );
      return;
    }

    final newProject = ProjectEntity(
      uuid: const Uuid().v4(),
      title: _titleController.text,
      // description: _descriptionController.text,
    );

    ref.read(projectRepositoryProvider).put(newProject);

    // Navigate back to projects
    if (context.router.canPop()) {
      context.router.pop();
    } else {
      context.gotoProjects();
    }
  }
}
