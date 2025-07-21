import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'thing_repository.dart';
import '../../things/activity/activity.entity.dart';
import '../objectbox.g.dart';

/// Provider for the Activity repository
final activityRepositoryProvider = Provider<ActivityRepository>((ref) {
  return ActivityRepository(ref);
});

/// Repository for Activity entities
/// Provides domain-specific query methods for activities
class ActivityRepository extends ThingRepository<ActivityEntity> {
  ActivityRepository(super.ref);

  @override
  Box<ActivityEntity> get box => obx.store.box<ActivityEntity>();

  @override
  String get type => 'activity';

  /// Get active (currently running) activities
  List<ActivityEntity> active() {
    return withStatus(ActivityStatus.active);
  }

  /// Add a new activity
  int add(ActivityEntity activity) {
    return box.put(activity);
  }

  /// Archive an activity (mark as archived status if such exists)
  void archive(String uuid) {
    final activity = getByUUID(uuid);
    if (activity != null) {
      // For now, we'll just mark as completed
      // In the future, we might add an archived status
      activity.status = ActivityStatus.completed;
      update(activity);
    }
  }

  /// Get completed activities
  List<ActivityEntity> completed() {
    return withStatus(ActivityStatus.completed);
  }

  /// Delete an activity by ID
  bool delete(int id) {
    return box.remove(id);
  }

  /// Delete an activity by UUID
  bool deleteByUUID(String uuid) {
    final activity = getByUUID(uuid);
    if (activity != null) {
      return delete(activity.id);
    }
    return false;
  }

  /// Get activities linked to a specific task
  List<ActivityEntity> forTask(String taskUuid) {
    // Filter at application level since we need to check ToOne relationship
    return getAll()
        .where(
          (activity) =>
              activity.task.hasValue && activity.task.target?.uuid == taskUuid,
        )
        .toList();
  }

  /// Get activities within a date range
  List<ActivityEntity> inDateRange(DateTime start, DateTime end) {
    final query = box.query(
      ActivityEntity_.startDate.between(
        start.millisecondsSinceEpoch,
        end.millisecondsSinceEpoch,
      ),
    );
    return query.build().find();
  }

  /// Get activities for today
  List<ActivityEntity> today() {
    final now = DateTime.now();
    final startOfDay = DateTime(now.year, now.month, now.day);
    final endOfDay = startOfDay.add(const Duration(days: 1));
    return inDateRange(startOfDay, endOfDay);
  }

  /// Update an existing activity
  int update(ActivityEntity activity) {
    return box.put(activity);
  }

  /// Get activities by kind
  List<ActivityEntity> withKind(ActivityKind kind) {
    final query = box.query(ActivityEntity_.dbKind.equals(kind.index));
    return query.build().find();
  }

  /// Get activities by status
  List<ActivityEntity> withStatus(ActivityStatus status) {
    final query = box.query(ActivityEntity_.dbStatus.equals(status.index));
    return query.build().find();
  }

  /// Get activities by tags
  List<ActivityEntity> withTag(String tag) {
    // For ObjectBox, we need to filter at application level since tags is a list
    return getAll()
        .where((activity) => activity.tags?.contains(tag) ?? false)
        .toList();
  }
}
