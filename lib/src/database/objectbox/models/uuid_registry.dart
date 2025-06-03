import 'package:objectbox/objectbox.dart';

@Entity()
class UuidRegistry {
  @Id()
  int id = 0;

  @Unique()
  String uuid;

  /// Type of entity: 'task', 'project', 'note', etc.
  @Index()
  String entityType;

  /// ObjectBox ID of the actual entity
  int entityId;

  DateTime createdAt;

  UuidRegistry({
    required this.uuid,
    required this.entityType,
    required this.entityId,
    DateTime? createdAt,
  }) : createdAt = createdAt ?? DateTime.now();
}
