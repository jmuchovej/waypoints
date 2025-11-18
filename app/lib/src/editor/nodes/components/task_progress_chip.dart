import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../styling/text_styles.dart';
import '../../../things/task/task.entity.dart';
import '../../../objectbox/services/repository_router.dart';

/// A pill-shaped widget that displays the completion progress of subtasks
/// in a Things 3-like style with a circular progress indicator and text.
class TaskProgressChip extends ConsumerWidget {
  const TaskProgressChip({super.key, required this.parentTask});

  final TaskEntity parentTask;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Get all subtasks for this parent task
    final resolver = ref.read(repositoryRouterProvider);
    final subtasks = resolver.resolveTypedChildren<TaskEntity>(
      parentTask.thing.children.toList(),
      TaskEntity.key,
    );

    // If no subtasks, don't show the pill
    if (subtasks.isEmpty) {
      return const SizedBox.shrink();
    }

    // Calculate completion statistics
    final completedCount = subtasks
        .where((task) => task.status.isCompleted)
        .length;
    final totalCount = subtasks.length;
    final progress = totalCount > 0 ? completedCount / totalCount : 0.0;

    final textStyles = WaypointsTextStyles.of(ref);
    final fontSize = textStyles.chipText.fontSize!;

    return Row(
      children: [
        Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            // Pie-chart style progress indicator
            SizedBox(
              width: 1.25 * fontSize,
              height: 1.25 * fontSize,
              child: CustomPaint(
                painter: _PieProgressPainter(
                  progress: progress,
                  backgroundColor: Theme.of(
                    context,
                  ).colorScheme.outline.withValues(alpha: 0.2),
                  progressColor: progress == 1.0
                      ? Theme.of(context).colorScheme.primary
                      : Theme.of(context).colorScheme.secondary,
                ),
              ),
            ),
            SizedBox(width: 0.5 * fontSize),
            // Progress text
            Text(
              '${completedCount}/${totalCount}',
              style: Theme.of(context).textTheme.labelSmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
                fontSize: fontSize,
                fontWeight: FontWeight.w500,
              ),
            ),
          ],
        ),
      ],
    );
  }
}

/// Custom painter that creates a pie-chart style progress indicator
/// similar to Things 3's circular progress design.
class _PieProgressPainter extends CustomPainter {
  const _PieProgressPainter({
    required this.progress,
    required this.backgroundColor,
    required this.progressColor,
  });

  final double progress;
  final Color backgroundColor;
  final Color progressColor;

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final outerRadius = size.width / 2;
    final gap = 2.0; // Gap between outer ring and inner pie
    final innerRadius = outerRadius - gap - 1.0; // Account for stroke width

    // Paint for outer ring stroke
    final ringPaint = Paint()
      ..color = progressColor
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.0;

    // Paint for inner background circle
    final innerBackgroundPaint = Paint()
      ..color = backgroundColor.withValues(alpha: 0.3)
      ..style = PaintingStyle.fill;

    // Paint for progress pie
    final progressPaint = Paint()
      ..color = progressColor
      ..style = PaintingStyle.fill;

    // Draw outer ring
    canvas.drawCircle(center, outerRadius - 0.5, ringPaint);

    // Draw inner background circle
    canvas.drawCircle(center, innerRadius, innerBackgroundPaint);

    // Draw progress pie slice (inside the ring)
    if (progress > 0) {
      final sweepAngle = 2 * 3.14159 * progress; // Convert to radians
      const startAngle = -3.14159 / 2; // Start from top (12 o'clock)

      final rect = Rect.fromCircle(center: center, radius: innerRadius);

      // Create path for pie slice
      final path = Path()
        ..moveTo(center.dx, center.dy) // Move to center
        ..arcTo(rect, startAngle, sweepAngle, false) // Draw arc
        ..close(); // Close path back to center

      canvas.drawPath(path, progressPaint);
    }
  }

  @override
  bool shouldRepaint(covariant _PieProgressPainter oldDelegate) {
    return oldDelegate.progress != progress ||
        oldDelegate.backgroundColor != backgroundColor ||
        oldDelegate.progressColor != progressColor;
  }
}
