import 'package:objectbox/objectbox.dart';
import 'task.dart';

@Entity()
class ProjectEntity {
  @Id()
  int id = 0;

  String uuid;
  String name;
  String description;

  @Backlink('project')
  final tasks = ToMany<TaskEntity>();

  final parent = ToOne<ProjectEntity>();
  @Backlink()
  final children = ToMany<ProjectEntity>();

  ProjectEntity({
    required this.uuid,
    required this.name,
    required this.description,
  });

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! ProjectEntity) return false;
    return id == other.id && uuid == other.uuid;
  }
}
