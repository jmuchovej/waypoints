import 'package:flutter/material.dart';
import 'task.entity.dart';

/// Represents the urgency calculation result for a task with detailed breakdown.
///
/// This class provides structured access to urgency components and calculations,
/// making it easier to work with urgency data in the UI and business logic.
class TaskUrgency implements Comparable<TaskUrgency> {
  /// Create a TaskUrgency calculation for the given task.
  factory TaskUrgency.calculate(
    TaskEntity task, {
    UrgencyCoefficients? coefficients,
  }) {
    coefficients = coefficients ?? UrgencyCoefficients.defaultValues();

    // Calculate individual components
    final priorityContrib = _calculatePriorityUrgency(task, coefficients);
    final dueDateContrib = _calculateDueDateUrgency(task, coefficients);
    final ageContrib = _calculateAgeUrgency(task, coefficients);
    final statusContrib = _calculateStatusUrgency(task, coefficients);
    final projectContrib = _calculateProjectUrgency(task, coefficients);

    // Future components (not yet implemented)
    const tagsContrib = 0.0;
    const blockingContrib = 0.0;
    const scheduledContrib = 0.0;
    const annotationsContrib = 0.0;

    final totalUrgency =
        priorityContrib +
        dueDateContrib +
        ageContrib +
        statusContrib +
        projectContrib +
        tagsContrib +
        blockingContrib +
        scheduledContrib +
        annotationsContrib;

    return TaskUrgency._(
      total: totalUrgency,
      priority: priorityContrib,
      dueDate: dueDateContrib,
      age: ageContrib,
      status: statusContrib,
      project: projectContrib,
      tags: tagsContrib,
      blocking: blockingContrib,
      scheduled: scheduledContrib,
      annotations: annotationsContrib,
      task: task,
      coefficients: coefficients,
    );
  }

  TaskUrgency._({
    required this.total,
    required this.priority,
    required this.dueDate,
    required this.age,
    required this.status,
    required this.project,
    required this.tags,
    required this.blocking,
    required this.scheduled,
    required this.annotations,
    required this.task,
    required this.coefficients,
  });

  /// The task this urgency calculation is for.
  final TaskEntity task;

  /// The coefficients used for this calculation.
  final UrgencyCoefficients coefficients;

  /// Total urgency score (sum of all components).
  final double total;

  /// Individual urgency components
  final double priority;
  final double dueDate;
  final double age;
  final double status;
  final double project;
  final double tags;
  final double blocking;
  final double scheduled;
  final double annotations;

  /// Get urgency level classification.
  UrgencyLevel get level => UrgencyLevel.fromScore(total);

  /// Get a human-readable urgency description.
  String get description => level.description;

  /// Comparison operators for comparing urgency between tasks.
  bool operator >(TaskUrgency other) => total > other.total;
  bool operator <(TaskUrgency other) => total < other.total;
  bool operator >=(TaskUrgency other) => total >= other.total;
  bool operator <=(TaskUrgency other) => total <= other.total;

  /// Get the dominant urgency factor (highest contributing component).
  UrgencyFactor get dominantFactor {
    final components = {
      UrgencyFactor.priority: priority,
      UrgencyFactor.dueDate: dueDate,
      UrgencyFactor.age: age,
      UrgencyFactor.status: status,
      UrgencyFactor.project: project,
      UrgencyFactor.tags: tags,
      UrgencyFactor.blocking: blocking,
      UrgencyFactor.scheduled: scheduled,
      UrgencyFactor.annotations: annotations,
    };

    return components.entries.reduce((a, b) => a.value > b.value ? a : b).key;
  }

  @override
  String toString() =>
      'TaskUrgency(total: ${total.toStringAsFixed(2)}, level: ${level.name})';

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    return other is TaskUrgency &&
        other.task == task &&
        (other.total - total).abs() <
            0.001; // Allow for small floating point differences
  }

  @override
  int get hashCode => Object.hash(task, total.round());

  // Private calculation methods
  static double _calculatePriorityUrgency(
    TaskEntity task,
    UrgencyCoefficients coefficient,
  ) {
    switch (task.priority) {
      case TaskPriority.H:
        return coefficient.priorityH;
      case TaskPriority.M:
        return coefficient.priorityM;
      case TaskPriority.L:
        return coefficient.priorityL;
    }
  }

  static double _calculateDueDateUrgency(
    TaskEntity task,
    UrgencyCoefficients coefficient,
  ) {
    final now = DateTime.now();
    final due = task.dueDate;

    // Calculate precise time until due (in hours for better granularity)
    final hoursUntilDue = due.difference(now).inHours;
    final daysUntilDue = hoursUntilDue / 24.0;

    if (hoursUntilDue < 0) {
      // Overdue - maximum urgency
      return coefficient.due;
    } else if (hoursUntilDue <= 24) {
      // Due within 24 hours - very high urgency
      return coefficient.due * (1.0 - (hoursUntilDue / 48.0));
    } else if (daysUntilDue <= 7) {
      // Due within a week - scaled urgency
      return coefficient.due * (0.5 - (daysUntilDue - 1) / 14.0);
    } else {
      // Due far in future - minimal urgency
      return coefficient.due * 0.05;
    }
  }

  static double _calculateAgeUrgency(
    TaskEntity task,
    UrgencyCoefficients coefficient,
  ) {
    final age = DateTime.now().difference(task.createDate).inDays;
    final maxAge = coefficient.ageMaxDays;

    final ageRatio = (age / maxAge).clamp(0.0, 1.0);
    return coefficient.age * ageRatio;
  }

  static double _calculateStatusUrgency(
    TaskEntity task,
    UrgencyCoefficients coefficients,
  ) {
    switch (task.status) {
      case TaskStatus.started:
        return coefficients.active;
      case TaskStatus.waiting:
        return coefficients.waiting;
      case TaskStatus.stopped:
      case TaskStatus.dropped:
      case TaskStatus.complete:
        return coefficients.blocked;
      default:
        return 0.0;
    }
  }

  static double _calculateProjectUrgency(
    TaskEntity task,
    UrgencyCoefficients coefficients,
  ) {
    return task.project.hasValue ? coefficients.project : 0.0;
  }

  @override
  int compareTo(TaskUrgency other) => level.compareTo(other.level);
}

/// Urgency level classifications for easy categorization.
enum UrgencyLevel implements Comparable<UrgencyLevel> {
  critical(
    15,
    color: Colors.red,
    icon: Icons.priority_high_rounded,
    description: 'Critical - Needs immediate attention',
  ),
  high(
    10,
    color: Colors.orange,
    icon: Icons.warning_rounded,
    description: 'High - Should be prioritized',
  ),
  medium(
    5,
    color: Colors.yellow,
    icon: Icons.schedule_rounded,
    description: 'Medium - Normal priority',
  ),
  low(
    1,
    color: Colors.blue,
    icon: Icons.info_rounded,
    description: 'Low - Can be deferred',
  ),
  minimal(
    0,
    color: Colors.green,
    icon: Icons.low_priority_rounded,
    description: 'Minimal - Background task',
  );

  const UrgencyLevel(
    this.threshold, {
    required this.color,
    required this.icon,
    required this.description,
  });

  factory UrgencyLevel.fromScore(num score) {
    return UrgencyLevel.values.where((level) => score >= level.threshold).first;
  }

  final int threshold;
  final Color color;
  final IconData icon;
  final String description;

  @override
  int compareTo(UrgencyLevel other) => threshold - other.threshold;
}

/// Individual urgency factors for analysis.
enum UrgencyFactor {
  priority('Priority'),
  dueDate('Due Date'),
  age('Age'),
  status('Status'),
  project('Project'),
  tags('Tags'),
  blocking('Blocking'),
  scheduled('Scheduled'),
  annotations('Annotations');

  const UrgencyFactor(this.name);
  final String name;
}

/// Configuration class for urgency calculation coefficients.
class UrgencyCoefficients {
  /// Default Taskwarrior coefficients.
  factory UrgencyCoefficients.defaultValues() {
    return const UrgencyCoefficients(
      priorityH: 6.0,
      priorityM: 3.9,
      priorityL: 1.8,
      due: 12.0,
      age: 2.0,
      ageMaxDays: 365,
      active: 4.0,
      waiting: -3.0,
      blocked: -5.0,
      project: 1.0,
      blocking: 8.0,
      scheduled: 5.0,
      annotations: 1.0,
      tags: 1.0,
      nextTag: 15.0,
    );
  }

  /// Custom coefficients optimized for personal productivity.
  factory UrgencyCoefficients.personalOptimized() {
    return const UrgencyCoefficients(
      priorityH: 10.0,
      priorityM: 5.0,
      priorityL: 2.0,
      due: 15.0,
      age: 1.5,
      ageMaxDays: 180,
      active: 6.0,
      waiting: -2.0,
      blocked: -3.0,
      project: 2.0,
      blocking: 8.0,
      scheduled: 5.0,
      annotations: 1.0,
      tags: 1.0,
      nextTag: 20.0,
    );
  }

  const UrgencyCoefficients({
    required this.priorityH,
    required this.priorityM,
    required this.priorityL,
    required this.due,
    required this.age,
    required this.ageMaxDays,
    required this.active,
    required this.waiting,
    required this.blocked,
    required this.project,
    required this.blocking,
    required this.scheduled,
    required this.annotations,
    required this.tags,
    required this.nextTag,
  });

  /// Priority coefficients
  final double priorityH;
  final double priorityM;
  final double priorityL;

  /// Due date coefficient
  final double due;

  /// Age coefficients
  final double age;
  final int ageMaxDays;

  /// Status coefficients
  final double active;
  final double waiting;
  final double blocked;

  /// Project coefficient
  final double project;

  /// Task relationship coefficients
  final double blocking;
  final double scheduled;

  /// Metadata coefficients
  final double annotations;
  final double tags;
  final double nextTag;

  /// Copy this configuration with selective overrides.
  UrgencyCoefficients copyWith({
    double? priorityH,
    double? priorityM,
    double? priorityL,
    double? due,
    double? age,
    int? ageMaxDays,
    double? active,
    double? waiting,
    double? blocked,
    double? project,
    double? blocking,
    double? scheduled,
    double? annotations,
    double? tags,
    double? nextTag,
  }) {
    return UrgencyCoefficients(
      priorityH: priorityH ?? this.priorityH,
      priorityM: priorityM ?? this.priorityM,
      priorityL: priorityL ?? this.priorityL,
      due: due ?? this.due,
      age: age ?? this.age,
      ageMaxDays: ageMaxDays ?? this.ageMaxDays,
      active: active ?? this.active,
      waiting: waiting ?? this.waiting,
      blocked: blocked ?? this.blocked,
      project: project ?? this.project,
      blocking: blocking ?? this.blocking,
      scheduled: scheduled ?? this.scheduled,
      annotations: annotations ?? this.annotations,
      tags: tags ?? this.tags,
      nextTag: nextTag ?? this.nextTag,
    );
  }
}

/// Extension on TaskEntity to provide easy urgency access.
extension TaskUrgencyExtension on TaskEntity {
  /// Calculate urgency for this task and return a TaskUrgency object.
  TaskUrgency get urgency => urgencyWithCoefficients(
    coefficients: UrgencyCoefficients.defaultValues(),
  );

  TaskUrgency urgencyWithCoefficients({UrgencyCoefficients? coefficients}) {
    return TaskUrgency.calculate(this, coefficients: coefficients);
  }
}

/// Extension on [List<TaskEntity>] for urgency-based operations.
extension TaskUrgencyList on List<TaskEntity> {
  /// Sort tasks by urgency in descending order (most urgent first).
  List<TaskEntity> sortByUrgency({UrgencyCoefficients? coefficients}) {
    final sorted = List<TaskEntity>.from(this);
    sorted.sort((a, b) {
      final urgencyA = a.urgencyWithCoefficients(coefficients: coefficients);
      final urgencyB = b.urgencyWithCoefficients(coefficients: coefficients);
      return urgencyB.total.compareTo(urgencyA.total);
    });
    return sorted;
  }

  /// Filter tasks by minimum urgency threshold.
  List<TaskEntity> whereUrgencyAbove(
    double threshold, {
    UrgencyCoefficients? coefficients,
  }) {
    return where((task) {
      return task.urgencyWithCoefficients(coefficients: coefficients).total >
          threshold;
    }).toList();
  }

  /// Filter tasks by urgency level.
  List<TaskEntity> whereUrgencyLevel(
    UrgencyLevel level, {
    UrgencyCoefficients? coefficients,
  }) {
    return where((task) {
      return task.urgencyWithCoefficients(coefficients: coefficients).level ==
          level;
    }).toList();
  }

  /// Get the top N most urgent tasks.
  List<TaskEntity> getMostUrgent(
    int count, {
    UrgencyCoefficients? coefficients,
  }) {
    return sortByUrgency(coefficients: coefficients).take(count).toList();
  }

  /// Get urgency calculations for all tasks (useful for batch processing).
  List<TaskUrgency> getUrgencyCalculations({
    UrgencyCoefficients? coefficients,
  }) {
    return map(
      (task) => task.urgencyWithCoefficients(coefficients: coefficients),
    ).toList();
  }
}
