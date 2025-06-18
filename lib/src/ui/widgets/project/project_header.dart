import 'package:flutter/material.dart';
import '../../../editor/editor.dart';

class ProjectHeader extends StatelessWidget {
  const ProjectHeader({
    super.key,
    required this.titleController,
    required this.titleFocusNode,
    required this.taskCount,
  });

  final AttributedTextEditingController titleController;
  final FocusNode titleFocusNode;
  final int taskCount;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 24.0, vertical: 16.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Project title
          TempoTextField(
            focusNode: titleFocusNode,
            controller: titleController,
            hintText: AttributedText('Untitled Project'),
            textStyleBuilder: (attributions) {
              return TextStyle(
                fontSize: 32,
                fontWeight: FontWeight.w600,
                height: 1.2,
                color: theme.colorScheme.onSurface,
              );
            },
            hintStyleBuilder: (attributions) {
              return TextStyle(
                fontSize: 32,
                fontWeight: FontWeight.w600,
                height: 1.2,
                color: theme.colorScheme.onSurface.withValues(alpha: 0.3),
              );
            },
          ),
          const SizedBox(height: 8),
          // Project metadata
          Text(
            '$taskCount tasks',
            style: TextStyle(
              fontSize: 16,
              color: theme.colorScheme.onSurface.withValues(alpha: 0.7),
            ),
          ),
        ],
      ),
    );
  }
}
