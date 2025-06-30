import 'package:flutter/material.dart';

class EmptyState extends StatelessWidget {
  const EmptyState({
    super.key,
    required this.icon,
    required this.title,
    this.subtitle,
    required this.label,
    required this.onPressed,
  });

  final IconData icon;
  final String title;
  final String? subtitle;
  final String label;
  final Function() onPressed;

  @override
  Widget build(BuildContext context) {
    final iconWidget = Icon(
      icon,
      size: 80,
      color: ColorScheme.of(context).onSurfaceVariant.withValues(alpha: 0.6),
    );
    final titleStyle = TextTheme.of(
      context,
    ).headlineSmall?.copyWith(fontWeight: FontWeight.bold);
    final titleWidget = [SizedBox(height: 16), Text(title, style: titleStyle)];

    final subtitleStyle = TextTheme.of(
      context,
    ).bodyMedium?.copyWith(color: ColorScheme.of(context).onSurfaceVariant);
    final subtitleWidget =
        subtitle != null
            ? [
              const SizedBox(height: 8),
              Text(
                subtitle!,
                style: subtitleStyle,
                textAlign: TextAlign.center,
              ),
            ]
            : [SizedBox.shrink(), SizedBox.shrink()];
    final labelWidget = [
      const SizedBox(height: 24),
      ElevatedButton.icon(
        icon: Icon(Icons.add),
        label: Text(label),
        onPressed: onPressed,
      ),
    ];

    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          iconWidget,
          ...titleWidget,
          ...subtitleWidget,
          ...labelWidget,
        ],
      ),
    );
  }
}
