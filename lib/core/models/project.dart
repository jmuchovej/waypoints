import 'package:objectbox/objectbox.dart';
import 'package:tempo/core/models/task.dart';

@Entity()
class ProjectEntity {
  @Id()
  int id = 0;

  String uuid;
  String title;
  String description;

  @Backlink('project')
  final tasks = ToMany<TaskEntity>();

  final parent = ToOne<ProjectEntity>();
  @Backlink()
  final children = ToMany<ProjectEntity>();

  ProjectEntity({
    required this.uuid,
    required this.title,
    required this.description,
  });

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! ProjectEntity) return false;
    return id == other.id && uuid == other.uuid;
  }

  @override
  int get hashCode => uuid.hashCode ^ title.hashCode ^ description.hashCode;
}
