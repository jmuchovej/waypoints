import 'package:objectbox/objectbox.dart';

@Entity()
class ThingEntity {
  ThingEntity({
    required this.uuid,
    required this.type,
    required this.id,
    DateTime? createDate,
  }) : createDate = createDate ?? DateTime.now();

  @Id()
  int registryID = 0;

  @Unique()
  String uuid;

  /// Type of entity: 'task', 'project', 'note', etc.
  @Index()
  String type;

  /// ObjectBox's ID for this [ThingEntity]
  int id;

  @Property(type: PropertyType.dateNano)
  DateTime createDate;

  @Index()
  String? plainText;

  @override
  String toString() {
    return 'ThingEntity{id: $id, uuid: $uuid, type: $type}';
  }
}
