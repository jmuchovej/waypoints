import 'package:objectbox/objectbox.dart';
import 'task.dart';

@Entity()
class TaskList {
  @Id()
  int id = 0;

  String uuid;
  String name;
  String description;

  @Backlink('taskList')
  final tasks = ToMany<Task>();

  TaskList({required this.uuid, required this.name, required this.description});

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! TaskList) return false;
    return id == other.id && uuid == other.uuid;
  }
}
