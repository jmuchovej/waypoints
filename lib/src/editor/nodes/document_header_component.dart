import 'package:flutter/material.dart';

import '../editor.dart';
import '../../infrastructure/logging.dart';
import '../styling/text_style_builders.dart';

/// Visual component for document headers
class DocumentHeaderComponent extends StatelessWidget {
  const DocumentHeaderComponent({
    super.key,
    required this.componentKey,
    required this.title,
    this.subtitle,
    this.icon,
  });

  final GlobalKey componentKey;
  final AttributedText title;
  final AttributedText? subtitle;
  final String? icon;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    // Log when DocumentHeader widget builds
    tempoLog.fine(
      '🖼️ DocumentHeaderComponent building widget with title: "${title.toPlainText()}"',
    );

    return BoxComponent(
      key: componentKey,
      isVisuallySelectable: false, // Headers should not be selectable
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          // Icon (if provided)
          if (icon != null) ...[
            Icon(
              _getIconData(icon!),
              size: 32,
              color: theme.colorScheme.primary,
            ),
            const SizedBox(width: 16),
          ],

          // Title and subtitle
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                TextComponent(
                  text: title,
                  textStyleBuilder: headerTextStyleBuilder(context),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  IconData _getIconData(String iconName) {
    // Simple icon mapping - you can expand this
    switch (iconName) {
      case 'tasks':
        return Icons.format_list_bulleted_rounded;
      case 'projects':
        return Icons.folder_rounded;
      case 'notes':
        return Icons.notes_rounded;
      default:
        return Icons.article_rounded;
    }
  }
}
