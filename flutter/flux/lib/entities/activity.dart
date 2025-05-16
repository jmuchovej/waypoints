import 'package:objectbox/objectbox.dart';

import 'task.dart';

import 'activity/_kind.dart' hide assertStableEnumValues;
import 'activity/_kind.dart' as K show assertStableEnumValues;

import 'activity/_status.dart' hide assertStableEnumValues;
import 'activity/_status.dart' as S show assertStableEnumValues;

export 'activity/_status.dart' hide assertStableEnumValues;
export 'activity/_kind.dart' hide assertStableEnumValues;

@Entity()
class ActivityEntity {
  @Id()
  int id = 0;

  String uuid;
  String name;

  @Property(type: PropertyType.date)
  DateTime? startDate;

  @Property(type: PropertyType.date)
  DateTime? endDate;

  @Transient()
  ActivityKind kind;

  @Transient()
  ActivityStatus status;

  List<String>? tags;

  final task = ToOne<TaskEntity>();

  bool archived;
  bool deleted;

  ActivityEntity({
    required this.uuid,
    required this.name,
    this.startDate,
    this.endDate,
    this.kind = ActivityKind.activity,
    this.status = ActivityStatus.created,
    this.tags,
    this.archived = false,
    this.deleted = false,
  });

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! ActivityEntity) return false;
    return id == other.id && uuid == other.uuid;
  }

  int get dbKind {
    K.assertStableEnumValues();
    return kind.index;
  }

  set dbKind(int value) {
    assert(0 < value && value < ActivityKind.values.length);
    kind = ActivityKind.values[value];
  }

  int get dbStatus {
    S.assertStableEnumValues();
    return status.index;
  }

  set dbStatus(int value) {
    assert(0 < value && value < ActivityStatus.values.length);
    status = ActivityStatus.values[value];
  }
}
